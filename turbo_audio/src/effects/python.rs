use std::{
    path::PathBuf,
    sync::{Arc, Mutex, RwLock},
};

use turbo_plugin::Color;

use super::Effect;
use crate::audio::audio_processing::{AudioSignalProcessor, FftResult};

#[derive(Debug)]
pub struct PythonEffectError {}

#[derive(Debug)]
pub struct PythonEffect {
    raw_effect: turbo_plugin_python::PythonEffect,
}

#[derive(Debug)]
pub struct PythonEffectSettings {}

pub struct PythonEffectManager {
    fft_result: Arc<RwLock<FftResult>>,
    fake_fft_results: Arc<Mutex<Vec<i32>>>,
}

impl PythonEffect {
    pub fn tick(&mut self, leds: &mut [turbo_plugin::Color]) {
        pyo3::Python::with_gil(|gil| {
            let colors = self.raw_effect.colors.as_ref(gil);
            if colors.len() > leds.len() {
                colors.del_slice(leds.len(), colors.len()).unwrap();
            }

            if colors.len() < leds.len() {
                let missing_count = leds.len() - colors.len();
                for _ in 0..missing_count {
                    colors
                        .append(pyo3::Py::new(gil, turbo_plugin_python::Color::default()).unwrap())
                        .unwrap();
                }
            }
            let update_result = self
                .raw_effect
                .update_fn
                .call(gil, (colors,), None)
                .unwrap();

            let rust_colors: Vec<turbo_plugin_python::Color> = update_result.extract(gil).unwrap();
            if rust_colors.len() != leds.len() {
                return;
            }

            leds.copy_from_slice(
                &rust_colors
                    .iter()
                    .map(|color| Color {
                        r: color.red,
                        g: color.green,
                        b: color.blue,
                    })
                    .collect::<Vec<_>>(),
            );
        });
    }
}

impl PythonEffectManager {
    pub fn new(audio_processor: &AudioSignalProcessor) -> Self {
        println!("Creating python effect manager");
        let fft_results = Arc::new(Mutex::new(Vec::<i32>::default()));
        turbo_plugin_python::initialize_python_interpreter(fft_results.clone());
        Self {
            fft_result: audio_processor.fft_result.clone(),
            fake_fft_results: fft_results,
        }
    }

    pub fn create_effect(&self, file_path: &PathBuf) -> Result<Effect, PythonEffectError> {
        println!("Creating python effect: {:?}", file_path);
        let python_effect = turbo_plugin_python::PythonEffect::new(&file_path);
        Ok(Effect::Python(PythonEffect {
            raw_effect: python_effect,
        }))
    }
}
