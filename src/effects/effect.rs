use crate::core::color::Color;

pub trait Effect {    
    fn new() -> Self;
    
    // Depends on ownership
    fn update(&self, colors: &mut [Color]);
    // fn update();

    fn get_name(&self) -> &str;
    fn set_name(&mut self, name: String);

    // Dont ask me what box and dyn do
    // fn clone(&self) -> Box<dyn Effect>;


    fn get_id(&self) -> i64;
    fn get_settings_id(&self) -> i64;

    // Not necessary if the effect does not own its led?
    // I don't understand lifetime
    fn get_colors(&self) -> &[Color];


    fn serialize(&self) -> String;
    fn deserialize(data: &str) -> Self;

    fn set_number_of_leds(&mut self, size: usize);
    fn get_number_of_leds(&self) -> usize;
}
