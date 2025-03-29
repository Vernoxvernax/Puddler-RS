use crossterm::{
  execute,
  style::{Attribute, Color, Colors, Print, ResetColor, SetAttribute, SetColors},
};
use std::io::{stderr, stdout};

pub enum PrintMessageType {
  Warning,
  Error,
  Success,
}

pub const INVALID_INPUT: &str = "Invalid input. Please try again.";

pub fn print_message(message_type: PrintMessageType, message: &str) {
  execute!(stdout(), SetAttribute(Attribute::Bold)).unwrap();
  match message_type {
    PrintMessageType::Error => {
      execute!(
        stderr(),
        SetColors(Colors::new(Color::Red, Color::Reset)),
        Print("Error: ".to_string())
      )
      .unwrap();
    },
    PrintMessageType::Warning => {
      execute!(
        stdout(),
        SetColors(Colors::new(Color::Blue, Color::Reset)),
        Print("Warning: ".to_string())
      )
      .unwrap();
    },
    PrintMessageType::Success => {
      execute!(
        stdout(),
        SetColors(Colors::new(Color::Green, Color::Reset)),
        Print("Success: ".to_string())
      )
      .unwrap();
    },
  }
  execute!(
    stdout(),
    ResetColor,
    SetAttribute(Attribute::Bold),
    Print(message.to_string()),
    ResetColor,
  )
  .unwrap();
  println!();
}
