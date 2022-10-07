use futures_util::{StreamExt, FutureExt};
use warp::Filter;

pub async fn main() {
    let routes = warp::path("echo")
        .and(warp::ws())
        .map(|ws: warp::ws::Ws| {
            ws.on_upgrade(|websocket| {
                let (tx, rx) = websocket.split();
                rx.forward(tx).map(|result| {
                    if let Err(e) = result {
                        println!("Websocket error: {:?}", e);
                    }
                })
            })
        });

    warp::serve(routes).run(([127, 0, 0, 1], 9001)).await;
}

