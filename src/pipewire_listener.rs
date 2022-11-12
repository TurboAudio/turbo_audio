use pipewire::{
    prelude::ReadableDict,
    registry::{GlobalObject, Registry},
    spa::ForeignDict,
    Core, MainLoop,
};

use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
    sync::{Arc, Mutex},
    thread,
};

use anyhow::{anyhow, bail, ensure, Context, Result};

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
enum PortType {
    Input,
    Output,
}

#[derive(Debug, Clone, Copy)]
struct PipewireLink {
    id: u32,
    output_port_id: u32,
    input_port_id: u32,
    output_node_id: u32,
    input_node_id: u32,
}

#[derive(Debug)]
struct PipewirePort {
    name: String,
    id: u32,
    node_id: u32,
    port_type: PortType,
    links: HashSet<u32>,
}

struct PipewireNode {
    name: String,
    id: u32,
    input_ports: HashSet<u32>,
    output_ports: HashSet<u32>,
}

enum PipewireObjectType {
    Node,
    Port,
    Link,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum PortConnections {
    AllInOrder,
    Only(Vec<(String, String)>),
}

#[derive(Debug, Clone)]
pub struct StreamConnections {
    pub output_stream: String,
    pub input_stream: String,
    pub port_connections: PortConnections,
}

#[derive(Debug)]
pub struct StreamDescriptor {
    pub name: String,
    pub inputs: Vec<String>,
    pub ouputs: Vec<String>,
}

pub struct PipewireController {
    sender: pipewire::channel::Sender<Vec<StreamConnections>>,
    state: Arc<Mutex<PipewireState>>,
}

impl PipewireController {
    pub fn new() -> Self {
        let (sender, receiver) = pipewire::channel::channel();
        let state = Arc::default();
        thread::spawn({
            let state = Arc::clone(&state);
            move || pipewire_thread(vec![], receiver, Arc::clone(&state))
        });
        PipewireController {
            sender,
            state: Arc::clone(&state),
        }
    }

    pub fn set_stream_connections(&self, stream_connections: Vec<StreamConnections>) -> Result<()> {
        let result = self.sender.send(stream_connections);
        if result.is_err() {
            return Err(anyhow!("Failed to change pipewire connections"));
        }
        Ok(())
    }

    pub fn get_streams(&self) -> Result<Vec<StreamDescriptor>> {
        let mut stream_descriptors = Vec::new();
        let state = self.state.lock().unwrap();
        for node in state.nodes.values() {
            let mut descriptor = StreamDescriptor {
                name: node.name.clone(),
                inputs: vec![],
                ouputs: vec![],
            };
            for input_id in &node.input_ports {
                let port = state.ports.get(input_id).context("Port doesn't exist")?;
                descriptor.inputs.push(port.name.clone());
            }
            for output_id in &node.output_ports {
                let port = state.ports.get(output_id).context("Port doesn't exist")?;
                descriptor.ouputs.push(port.name.clone());
            }
            stream_descriptors.push(descriptor);
        }
        Ok(stream_descriptors)
    }
}

fn pipewire_thread(
    stream_connections: Vec<StreamConnections>,
    receiver: pipewire::channel::Receiver<Vec<StreamConnections>>,
    state: Arc<Mutex<PipewireState>>,
) -> Result<()> {
    let stream_connections = Rc::new(RefCell::new(stream_connections));

    let mainloop = MainLoop::new().context("Couldn't create pipewire mainloop")?;
    let context = pipewire::Context::new(&mainloop).context("Coudln't create pipewire context")?;
    let core = Rc::new(
        context
            .connect(None)
            .context("Couldn't create pipewire core")?,
    );
    let registry = Rc::new(
        core.get_registry()
            .context("Couldn't create pipewire registry")?,
    );

    let _listener = registry
        .add_listener_local()
        .global({
            let state = state.clone();
            let stream_connections = stream_connections.clone();
            let core = core.clone();
            let registry = registry.clone();
            move |global| match global.type_ {
                pipewire::types::ObjectType::Node => {
                    let mut state = state.lock().unwrap();
                    state
                        .add_node(global)
                        .unwrap_or_else(|err| println!("{}", err));
                }
                pipewire::types::ObjectType::Port => {
                    let mut state = state.lock().unwrap();
                    state
                        .add_port(global)
                        .unwrap_or_else(|err| println!("{}", err));
                    add_missing_connections(&core, &state, &stream_connections.borrow_mut());
                }
                pipewire::types::ObjectType::Link => {
                    let mut state = state.lock().unwrap();
                    state
                        .add_link(global)
                        .unwrap_or_else(|err| println!("{}", err));
                    if let Some(new_link) = state.links.get(&global.id) {
                        check_remove_link(
                            &state,
                            &registry,
                            new_link,
                            &stream_connections.borrow_mut(),
                        )
                        .unwrap_or_else(|err| println!("{}", err));
                    }
                }
                _ => {}
            }
        })
        .global_remove({
            let stream_connections = stream_connections.clone();
            let state = state.clone();
            let core = core.clone();
            move |id| {
                let mut state = state.lock().unwrap();
                state
                    .remove_object(id)
                    .unwrap_or_else(|err| println!("{}", err));
                add_missing_connections(&core, &state, &stream_connections.borrow_mut());
            }
        })
        .register();

    let _receiver = receiver.attach(&mainloop, {
        move |new_stream_connections| {
            let state = state.lock().unwrap();
            *stream_connections.borrow_mut() = new_stream_connections;
            for link in state.links.values().clone() {
                check_remove_link(&state, &registry, link, &stream_connections.borrow_mut())
                    .unwrap_or_else(|err| println!("{}", err));
            }
            add_missing_connections(&core, &state, &stream_connections.borrow_mut());
        }
    });

    mainloop.run();
    Ok(())
}

#[derive(Default)]
struct PipewireState {
    nodes: HashMap<u32, PipewireNode>,
    ports: HashMap<u32, PipewirePort>,
    links: HashMap<u32, PipewireLink>,
    output_to_input_port_links: HashMap<u32, HashSet<u32>>,
    id_types: HashMap<u32, PipewireObjectType>,
    node_name_to_ids: HashMap<String, HashSet<u32>>,
}

impl PipewireState {
    fn get_connected_port_ids_between_node_ids(
        &self,
        output_node_id: &u32,
        input_node_id: &u32,
    ) -> Option<Vec<(u32, u32, u32, u32)>> {
        if let (Some(output_node), Some(input_node)) = (
            self.nodes.get(output_node_id),
            self.nodes.get(input_node_id),
        ) {
            let mut connections = Vec::<(u32, u32, u32, u32)>::new();
            for output_port in &output_node.output_ports {
                if let Some(connected_input_ports) =
                    self.output_to_input_port_links.get(output_port)
                {
                    for input_port in &input_node.input_ports {
                        if connected_input_ports.contains(input_port) {
                            connections.push((
                                *output_port,
                                *input_port,
                                *output_node_id,
                                *input_node_id,
                            ));
                        }
                    }
                }
            }
            return Some(connections);
        }
        None
    }

    fn get_connected_port_ids_between_node_names(
        &self,
        output_node_name: &str,
        input_node_name: &str,
    ) -> Option<Vec<(u32, u32, u32, u32)>> {
        if let (Some(output_node_ids), Some(input_node_ids)) = (
            self.node_name_to_ids.get(output_node_name),
            self.node_name_to_ids.get(input_node_name),
        ) {
            let mut connections = Vec::<(u32, u32, u32, u32)>::new();
            for output_node_id in output_node_ids.iter() {
                for input_node_id in input_node_ids.iter() {
                    if let Some(mut new_connections) =
                        self.get_connected_port_ids_between_node_ids(output_node_id, input_node_id)
                    {
                        connections.append(&mut new_connections);
                    }
                }
            }
            return Some(connections);
        }
        None
    }

    fn get_connected_port_names_between_node_names(
        &self,
        output_node_name: &str,
        input_node_name: &str,
    ) -> Option<HashMap<(String, String), (u32, u32)>> {
        let connections =
            self.get_connected_port_ids_between_node_names(output_node_name, input_node_name)?;
        let mut connected_port_names = HashMap::<(String, String), (u32, u32)>::new();
        for (output_port_id, input_port_id, output_node_id, input_node_id) in connections {
            if let (Some(output_port), Some(input_port)) = (
                self.ports.get(&output_port_id),
                self.ports.get(&input_port_id),
            ) {
                connected_port_names.insert(
                    (output_port.name.clone(), input_port.name.clone()),
                    (output_node_id, input_node_id),
                );
            }
        }
        Some(connected_port_names)
    }

    fn add_node(&mut self, node: &GlobalObject<ForeignDict>) -> Result<()> {
        let props = node
            .props
            .as_ref()
            .context("Node object doesn't have properties")?;

        let name = props
            .get("node.nick")
            .or_else(|| props.get("node.description"))
            .or_else(|| props.get("node.name"))
            .unwrap_or_default()
            .to_string();

        self.nodes.insert(
            node.id,
            PipewireNode {
                id: node.id,
                name: name.clone(),
                input_ports: HashSet::new(),
                output_ports: HashSet::new(),
            },
        );

        self.id_types.insert(node.id, PipewireObjectType::Node);
        self.node_name_to_ids
            .entry(name)
            .or_default()
            .insert(node.id);

        Ok(())
    }

    fn add_port(&mut self, port: &GlobalObject<ForeignDict>) -> Result<()> {
        let props = port
            .props
            .as_ref()
            .context("Port object doesn't have properties")?;

        let name = props.get("port.name").unwrap_or_default().to_string();

        let node_id = props
            .get("node.id")
            .context("Port object doesn't have node.id property")?
            .parse::<u32>()
            .context("Couldn't parse node.id as u32")?;

        let port_type = match props.get("port.direction") {
            Some("in") => PortType::Input,
            Some("out") => PortType::Output,
            Some(port_type) => {
                bail!("Port direction not supported: {:?}", port_type);
            }
            _ => {
                bail!("No port direction");
            }
        };

        let node = self.nodes.get_mut(&node_id).with_context(|| {
            format!(
                "Failed to add port #{} because it's parent node #{} was never created",
                port.id, node_id
            )
        })?;

        self.ports.insert(
            port.id,
            PipewirePort {
                name,
                id: port.id,
                node_id,
                port_type,
                links: HashSet::new(),
            },
        );

        self.id_types.insert(port.id, PipewireObjectType::Port);

        match port_type {
            PortType::Input => {
                node.input_ports.insert(port.id);
            }
            PortType::Output => {
                node.output_ports.insert(port.id);
            }
        }
        Ok(())
    }

    fn add_link(&mut self, link: &GlobalObject<ForeignDict>) -> Result<()> {
        let props = link
            .props
            .as_ref()
            .context("Link object doesn't have properties")?;

        let output_port_id = props
            .get("link.output.port")
            .context("No output port for link")?
            .to_string()
            .parse::<u32>()
            .context("Failed to parse u32")?;

        let input_port_id = props
            .get("link.input.port")
            .context("No input port for link")?
            .to_string()
            .parse::<u32>()
            .context("Failed to parse u32")?;

        let output_node_id = props
            .get("link.output.node")
            .context("No input port for link")?
            .to_string()
            .parse::<u32>()
            .context("Failed to parse u32")?;

        let input_node_id = props
            .get("link.input.node")
            .context("No input port for link")?
            .to_string()
            .parse::<u32>()
            .context("Failed to parse u32")?;

        let output_port = self
            .ports
            .get_mut(&output_port_id)
            .context("Port was never registered")?;
        output_port.links.insert(link.id);

        let input_port = self
            .ports
            .get_mut(&input_port_id)
            .context("Port was never registered")?;
        input_port.links.insert(link.id);

        self.output_to_input_port_links
            .entry(output_port_id)
            .or_default()
            .insert(input_port_id);

        self.links.insert(
            link.id,
            PipewireLink {
                id: link.id,
                output_port_id,
                input_port_id,
                output_node_id,
                input_node_id,
            },
        );
        self.id_types.insert(link.id, PipewireObjectType::Link);

        Ok(())
    }

    fn remove_object(&mut self, id: u32) -> Result<()> {
        let pipewire_object_type = self
            .id_types
            .remove(&id)
            .with_context(|| format!("Couldn't remove object with id #{}", id))?;
        match pipewire_object_type {
            PipewireObjectType::Node => self.remove_node(id),
            PipewireObjectType::Port => self.remove_port(id),
            PipewireObjectType::Link => self.remove_link(id),
        }
    }

    fn remove_node(&mut self, id: u32) -> Result<()> {
        let node = self
            .nodes
            .remove(&id)
            .with_context(|| format!("Node with id {} doesn't exist", id))?;
        let ids = self.node_name_to_ids.get_mut(&node.name).with_context(|| {
            format!("Error while removing node #{}, id not mapped to a name", id)
        })?;
        ensure!(
            ids.remove(&id),
            "Error while removing node #{}, id not mapped to a name",
            id
        );
        if ids.is_empty() {
            self.node_name_to_ids.remove(&node.name);
        }
        Ok(())
    }

    fn remove_port(&mut self, id: u32) -> Result<()> {
        let port = self
            .ports
            .remove(&id)
            .with_context(|| format!("Error removing port #{}, port doesn't exist", id))?;

        let parent_node = self.nodes.get_mut(&port.node_id).with_context(|| {
            format!(
                "Error removing port #{}, parent node #{} doesn't exist",
                id, port.node_id
            )
        })?;

        match port.port_type {
            PortType::Input => {
                ensure!(
                    parent_node.input_ports.remove(&id),
                    "Error removing port #{}. Parent node #{} didn't have it as an input port",
                    id,
                    port.node_id
                );
            }
            PortType::Output => {
                ensure!(
                    parent_node.output_ports.remove(&id),
                    "Error while removing port #{}, parent node #{} \
                    didn't have it as an output port",
                    id,
                    port.node_id
                );
            }
        }
        Ok(())
    }

    fn remove_link(&mut self, id: u32) -> Result<()> {
        fn remove_link_from_port(
            state: &mut PipewireState,
            link_id: &u32,
            port_id: &u32,
        ) -> Result<()> {
            let port = state.ports.get_mut(port_id).with_context(|| {
                format!(
                    "Error while removing link #{}, input port #{} doesn't exist",
                    link_id, port_id
                )
            })?;
            ensure!(
                port.links.remove(link_id),
                "Error while removing link #{}, input port #{} doesn't have it as a link",
                link_id,
                port_id
            );
            Ok(())
        }

        let link = self
            .links
            .remove(&id)
            .with_context(|| format!("Error while removing link #{}, link doesn't exist", id))?;

        remove_link_from_port(self, &id, &link.input_port_id)?;
        remove_link_from_port(self, &id, &link.output_port_id)?;

        let output_port_links = self
            .output_to_input_port_links
            .get_mut(&link.output_port_id)
            .with_context(|| {
                format!(
                    "Error while removing link #{}, link representation not present",
                    id
                )
            })?;

        ensure!(
            output_port_links.remove(&link.input_port_id),
            "Error while removing link #{}, link representation not present",
            id
        );

        if output_port_links.is_empty() {
            self.output_to_input_port_links.remove(&link.output_port_id);
        }

        Ok(())
    }
}

fn get_nodes<'a>(state: &'a PipewireState, stream_name: &str) -> Vec<&'a PipewireNode> {
    match state.node_name_to_ids.get(stream_name) {
        Some(node_ids) => node_ids
            .iter()
            .filter_map(|id| state.nodes.get(id))
            .collect(),
        _ => {
            vec![]
        }
    }
}

fn get_port_connections(
    state: &PipewireState,
    stream_connections: &StreamConnections,
) -> Vec<(String, String)> {
    match &stream_connections.port_connections {
        PortConnections::Only(port_connections) => port_connections.clone(),
        PortConnections::AllInOrder => {
            let output_nodes = get_nodes(state, &stream_connections.output_stream);
            let input_nodes = get_nodes(state, &stream_connections.input_stream);
            let mut port_connections = Vec::new();
            for input_node in input_nodes {
                let mut input_ports: Vec<String> = input_node
                    .input_ports
                    .clone()
                    .into_iter()
                    .filter_map(|port_id| state.ports.get(&port_id))
                    .map(|port| port.name.clone())
                    .collect();
                input_ports.sort();

                for output_node in &output_nodes {
                    let mut output_ports: Vec<String> = output_node
                        .output_ports
                        .clone()
                        .into_iter()
                        .filter_map(|port_id| state.ports.get(&port_id))
                        .map(|port| port.name.clone())
                        .collect();
                    output_ports.sort();

                    for (output_port_name, input_port_name) in
                        output_ports.iter().zip(input_ports.iter())
                    {
                        port_connections.push((output_port_name.clone(), input_port_name.clone()));
                    }
                }
            }
            port_connections
        }
    }
}

fn get_connection_details_from_port_names(
    state: &PipewireState,
    output_node_name: &str,
    input_node_name: &str,
    connection_port_names: &(String, String),
) -> Option<(u32, u32, u32, u32)> {
    let (output_port_name, input_port_name) = connection_port_names;

    let output_nodes = get_nodes(state, output_node_name);
    let input_nodes = get_nodes(state, input_node_name);

    for output_node in &output_nodes {
        for input_node in &input_nodes {
            let output_ports = Vec::from_iter(output_node.output_ports.to_owned())
                .iter()
                .filter_map(|port_id| state.ports.get(port_id))
                .filter(|port| port.name == *output_port_name)
                .map(|port| port.id)
                .collect::<Vec<_>>();

            let input_ports = Vec::from_iter(input_node.input_ports.to_owned())
                .iter()
                .filter_map(|port_id| state.ports.get(port_id))
                .filter(|port| port.name == *input_port_name)
                .map(|port| port.id)
                .collect::<Vec<_>>();

            if let (Some(output_port), Some(input_port)) =
                (output_ports.first(), input_ports.first())
            {
                return Some((*output_port, *input_port, output_node.id, input_node.id));
            }
        }
    }
    None
}

fn add_missing_connections(
    core: &Core,
    state: &PipewireState,
    stream_connections: &[StreamConnections],
) {
    for stream_connection in stream_connections {
        let desired_port_connections = get_port_connections(state, stream_connection);
        let present_port_connection = state.get_connected_port_names_between_node_names(
            &stream_connection.output_stream,
            &stream_connection.input_stream,
        );

        if let Some(present_port_connection) = present_port_connection {
            let connections_to_add: Vec<&(String, String)> = desired_port_connections
                .iter()
                .filter(|&port_output_input_pair| {
                    !present_port_connection.contains_key(port_output_input_pair)
                })
                .collect();

            for connection_to_add in connections_to_add {
                if let Some((output_port, input_port, output_node, input_node)) =
                    get_connection_details_from_port_names(
                        state,
                        &stream_connection.output_stream,
                        &stream_connection.input_stream,
                        connection_to_add,
                    )
                {
                    add_link(core, output_port, input_port, output_node, input_node);
                }
            }
        }
    }
}

fn check_remove_link(
    state: &PipewireState,
    registry: &Registry,
    link: &PipewireLink,
    stream_connections: &[StreamConnections],
) -> Result<()> {
    let mut should_remove_link = match state.nodes.get(&link.input_node_id) {
        Some(input_node) => stream_connections
            .iter()
            .map(|stream_connection| stream_connection.input_stream.clone())
            .any(|x| x == input_node.name),
        None => false,
    };

    if !should_remove_link {
        return Ok(());
    }

    for stream_connection in stream_connections {
        let desired_port_connections = get_port_connections(state, stream_connection);
        for desired_port_connection in desired_port_connections {
            if let Some((output_port, input_port, output_node, input_node)) =
                get_connection_details_from_port_names(
                    state,
                    &stream_connection.output_stream,
                    &stream_connection.input_stream,
                    &desired_port_connection,
                )
            {
                if (output_port, input_port, output_node, input_node)
                    == (
                        link.output_port_id,
                        link.input_port_id,
                        link.output_node_id,
                        link.input_node_id,
                    )
                {
                    should_remove_link = false;
                    break;
                }
            }
        }
    }
    if should_remove_link {
        remove_link(link.id, registry)?;
    }
    Ok(())
}

fn add_link(core: &Core, output_port: u32, input_port: u32, output_node: u32, input_node: u32) {
    core.create_object::<pipewire::link::Link, _>(
        "link-factory",
        &pipewire::properties! {
            "link.input.port" => input_port.to_string(),
            "link.output.port" => output_port.to_string(),
            "link.input.node" => input_node.to_string(),
            "link.output.node"=> output_node.to_string(),
            "object.linger" => "1"
        },
    )
    .expect("Failed to add new link");
}

fn remove_link(link_id: u32, registry: &Registry) -> Result<()> {
    registry
        .destroy_global(link_id)
        .into_result()
        .map_err(|spa_error| anyhow!(spa_error))?;
    Ok(())
}
