use std::{path::Path, sync::{Arc, Mutex}};

fn main() {
    let fft_results = Arc::new(Mutex::new(Vec::<i32>::default()));
    turbo_plugin_python::initialize_python_interpreter(fft_results.clone());
    let plugin_file = Path::new("/home/samichoulo/repos/turbo_python/scripts/test.py");
    let python_effect = turbo_plugin_python::PythonEffect::new(&plugin_file);
    fft_results.lock().unwrap().push(5);
    for i in 0..9 {
        fft_results.lock().unwrap().push(i);
        println!("{}", fft_results.lock().unwrap().iter().max()
            .or_else(|| Some(&0))
            .unwrap());
        pyo3::Python::with_gil(|gil| {
            let colors = python_effect.colors.as_ref(gil);
            let update_result = python_effect.update_fn.call(gil, (colors,), None).unwrap();
            let _rust_colors: Vec<turbo_plugin_python::Color> = update_result.extract(gil).unwrap();
            // println!("Update result in rust {:?}", rust_colors);
        });
    }
}
