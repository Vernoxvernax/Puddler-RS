use crate::{media_center::Series, printing::INVALID_INPUT, MenuOptions};
use crossterm::{
  cursor::{
    EnableBlinking, Hide, MoveLeft, MoveTo, MoveToColumn, MoveToNextLine, MoveToPreviousLine,
    RestorePosition, SavePosition, Show,
  },
  event::{poll, read, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
  execute,
  style::{Print, Stylize},
  terminal::{
    self, disable_raw_mode, enable_raw_mode, Clear, ClearType, DisableLineWrap, EnableLineWrap,
    EnterAlternateScreen, LeaveAlternateScreen,
  },
};
use std::{
  char,
  io::{self, prelude::*, stdin, stdout},
  process::exit,
  time::Duration,
};

trait CSplit {
  fn take_chars(&self, start: usize, end: usize) -> String;
  fn insert_char(&mut self, char_index: usize, ch: char);
  fn remove_char(&mut self, char_index: usize);
}

impl CSplit for String {
  /// Start and End are inclusive
  fn take_chars(&self, start: usize, end: usize) -> String {
    let chars: Vec<char> = self.chars().collect();
    let mut collect: bool = false;
    let mut collector: String = String::new();
    for (index, ch) in chars.iter().enumerate() {
      if index == start {
        collect = true;
      }
      if collect {
        collector.push(*ch);
      }
      if index == end {
        break;
      }
    }
    collector
  }

  fn insert_char(&mut self, char_index: usize, ch: char) {
    let byte_index = self
      .char_indices()
      .nth(char_index)
      .map(|(i, _)| i)
      .unwrap_or_else(|| self.len());

    self.insert(byte_index, ch);
  }

  fn remove_char(&mut self, char_index: usize) {
    let byte_index = self
      .char_indices()
      .nth(char_index)
      .map(|(i, _)| i)
      .unwrap_or_else(|| self.len());

    self.remove(byte_index);
  }
}

#[derive(Clone, PartialEq)]
pub enum InteractiveOptionType {
  Header,
  Button,
  Button5s,
  MultiButton,
  ListButtons,
  TextInput,
  Special,
}

#[derive(Clone)]
pub struct InteractiveOption {
  pub text: String,
  pub option_type: InteractiveOptionType,
}

#[derive(Clone, PartialEq)]
pub enum SeriesOptions {
  Back,
  Played,
  UnPlayed,
  Play,
}

fn string_to_range(input: String) -> Vec<usize> {
  let mut temp = String::new();
  let mut num_buffer = String::new();
  let mut indexes: Vec<usize> = vec![];
  let mut last_indexes: Vec<usize> = vec![];
  let mut range = false;

  for ch in input.chars() {
    if ch == ',' {
      if range {
        indexes.extend(last_indexes.clone());
        last_indexes.clear();
        num_buffer.clear();
        range = false;
      } else if !num_buffer.is_empty() {
        if let Ok(num) = num_buffer.parse() {
          indexes.push(num);
        }
        num_buffer.clear();
      }
    } else if ch == '-' {
      if !num_buffer.is_empty() {
        temp = num_buffer.clone();
        num_buffer.clear();
        range = true;
      }
    } else if let Some(_digit) = ch.to_digit(10) {
      num_buffer.push(ch);
    }

    if range && !num_buffer.is_empty() {
      last_indexes.clear();
      let start_num: usize = temp.parse().unwrap(); // if this panicks, kill the user
      let end_num: usize = num_buffer.parse().unwrap();
      for num in start_num..=end_num {
        last_indexes.push(num);
      }
    }
  }

  if !last_indexes.is_empty() {
    indexes.extend(last_indexes);
    num_buffer.clear();
  }

  if !num_buffer.is_empty() {
    if let Ok(num) = num_buffer.parse() {
      indexes.push(num);
    } else {
      num_buffer.clear();
    }
  }

  indexes.sort();

  let mut minimum: Vec<usize> = vec![];
  indexes.iter().for_each(|num| {
    if let Some(last) = minimum.last() {
      if last != num {
        minimum.push(*num);
      }
    } else {
      minimum.push(*num);
    }
  });
  minimum
}

struct Episode {
  title: String,
  watched: bool,
}

fn series_select(text: Vec<String>, episodes: Vec<Episode>) -> (SeriesOptions, Option<Vec<usize>>) {
  let mut stdout = stdout();
  stdout.flush().expect("Failed to flush stdout");
  let main_options = format!(
    "\nOptions: [{}oggle Watching-Status], [Left-Arrow to go back]",
    "T".bold().grey()
  );

  execute!(stdout, EnterAlternateScreen, DisableLineWrap, Hide).unwrap();

  let total_size = {
    let size = episodes.len();
    (size as f64).log10().floor() as usize + 1
  };
  let terminal_height = terminal::size().unwrap().1 as usize;
  let terminal_width = terminal::size().unwrap().0 as usize;
  if (terminal_height - 3) < text.len() {
    text[..terminal_height - 3]
      .iter()
      .for_each(|l| println!("{}", l));
    println!("{}[⇣]", " ".repeat(terminal_width - 5));
  } else {
    text.iter().for_each(|l| println!("{}", l));
  }

  let mut update = false;
  let mut input = String::new();
  print!(": ");
  print!("{}", main_options);
  let mut options = main_options.clone();
  stdout.flush().expect("Failed to flush stdout");
  enable_raw_mode().unwrap();
  let mut skip_lines = 0;
  let mut mode = SeriesOptions::Play;
  let mut index_selection: Vec<usize> = vec![];
  loop {
    if poll(Duration::from_millis(100)).unwrap() {
      if let Ok(Event::Key(KeyEvent {
        code,
        modifiers,
        state: _,
        kind: KeyEventKind::Press,
      })) = read()
      {
        if let KeyCode::Char(ch) = code {
          if ch.is_ascii_digit() {
            input.push(ch);
            update = true;
          } else if ch == 'c' && modifiers == KeyModifiers::CONTROL {
            execute!(stdout, LeaveAlternateScreen, EnableLineWrap, Show).unwrap();
            disable_raw_mode().unwrap();
            exit(1);
          } else if ch == 't' {
            if mode != SeriesOptions::Played {
              options = format!(
                "\nMode: [{}] (Press '{}' again toggle between Played and Un-Played)",
                "Played".bold(),
                ch.to_uppercase()
              );
              mode = SeriesOptions::Played;
              update = true;
            } else {
              options = format!(
                "\nMode: [{}] (Press '{}' again toggle between Played and Un-Played)",
                "Un-Played".bold(),
                ch.to_uppercase()
              );
              mode = SeriesOptions::UnPlayed;
              update = true;
            }
          } else if ch == '-'
            || ch == ',' && mode == SeriesOptions::Played
            || mode == SeriesOptions::UnPlayed
          {
            input.push(ch);
            update = true;
          }
        } else if KeyCode::Enter == code {
          if input.is_empty() {
            if index_selection.is_empty() {
              for (index, episode) in episodes.iter().enumerate() {
                if !episode.watched {
                  input = index.to_string();
                  break;
                }
              }
            } else {
              break;
            }
          }
          break;
        } else if KeyCode::Backspace == code {
          if input.is_empty() {
            mode = SeriesOptions::Play;
            options = main_options.clone();
          }
          input.pop();
          update = true;
        } else if KeyCode::Up == code {
          if skip_lines > 0 && text.len() > (terminal_height - 5) {
            skip_lines -= 1;
            update = true;
          }
        } else if KeyCode::Down == code {
          if text.len() > (terminal_height - 3) + skip_lines {
            skip_lines += 1;
            update = true;
          }
        } else if KeyCode::Left == code {
          input.clear();
          index_selection.clear();
          break;
        }
      }
    }
    if update {
      disable_raw_mode().unwrap();
      execute!(stdout, MoveTo(0, 0), Clear(ClearType::FromCursorDown)).unwrap();
      if (terminal_height - 5) < text.len() {
        text[skip_lines..(terminal_height - 3) + skip_lines]
          .iter()
          .for_each(|l| println!("{}", l));
        if skip_lines != terminal_height - 5 {
          println!("{}[⇣]", " ".repeat(terminal_width - 5));
        } else {
          println!();
        }
      } else {
        text.iter().for_each(|l| println!("{}", l));
      }
      if mode == SeriesOptions::Play {
        let selection = input.parse::<usize>().unwrap_or(episodes.len());
        print!(": {}", input);
        if !input.is_empty() && selection < episodes.len() && input.len() <= total_size {
          print!(
            "{} = {}",
            " ".repeat(total_size - input.len()),
            episodes[selection].title
          );
        }
      } else if (input.ends_with(',') || input.ends_with('-')) && input.len() > 1 {
        print!(
          "[{}]: {}{}",
          "T".bold().grey(),
          &input[..input.len() - 1],
          input[input.len() - 1..].to_string().underlined()
        );
      } else {
        print!("[{}]: {}", "T".bold().grey(), input);
        let range = string_to_range(input.clone());
        print!(" {:?}", range);
        index_selection = range;
      }
      print!("{}", options);
      stdout.flush().expect("Failed to flush stdout");
      enable_raw_mode().unwrap();
      update = false;
    }
  }
  execute!(stdout, LeaveAlternateScreen, EnableLineWrap, Show).unwrap();
  disable_raw_mode().unwrap();
  if input.is_empty() && index_selection.is_empty() {
    (SeriesOptions::Back, None)
  } else if index_selection.is_empty() {
    if let Ok(num) = input.parse::<usize>() {
      (mode, Some(vec![num]))
    } else {
      (SeriesOptions::Back, None)
    }
  } else {
    (mode, Some(index_selection))
  }
}

pub fn plex_series_select(
  text: Vec<String>,
  series: crate::plex::Series,
) -> (SeriesOptions, Option<Vec<usize>>) {
  let mut episodes: Vec<Episode> = vec![];
  for season in series.seasons.clone() {
    for episode in season.episodes {
      episodes.push(Episode {
        title: episode.to_string(),
        watched: episode.viewCount.is_some(),
      });
    }
  }

  series_select(text, episodes)
}

pub fn jelly_series_select(
  text: Vec<String>,
  series: Series,
) -> (SeriesOptions, Option<Vec<usize>>) {
  let mut episodes: Vec<Episode> = vec![];
  for season in series.seasons.clone() {
    for episode in season.episodes {
      episodes.push(Episode {
        title: episode.to_string(),
        watched: episode.UserData.Played,
      });
    }
  }

  series_select(text, episodes)
}

pub fn interactive_menuoption(options: Vec<MenuOptions>) -> MenuOptions {
  let mut choices: Vec<InteractiveOption> = vec![];
  for option in options.clone() {
    choices.append(&mut vec![InteractiveOption {
      text: option.to_string(),
      option_type: InteractiveOptionType::Button,
    }]);
  }

  let selection = interactive_select(choices);
  return options.get(selection.0 .0).unwrap().clone();
}

fn display_options(
  options: &[InteractiveOption],
  selected_index: (usize, usize),
  inputs: Vec<String>,
) -> (usize, usize) {
  let terminal_width = terminal::size().unwrap().0 as usize - 3;
  let terminal_height = terminal::size().unwrap().1 as usize - 3;
  let too_many_items = options.len() > terminal_height;
  let mut stop = false;
  let mut real_index: i16 = 0;
  let mut start = 0;
  let mut end = options.len();
  let mut index = 0;
  if too_many_items {
    end = terminal_height;
    if selected_index.0 + 5 > end {
      end = selected_index.0 + 5;
      start = end - terminal_height;
      if end >= options.len() {
        start = options.len() - terminal_height;
        end = options.len();
      }
    }
    index += start;
  }
  disable_raw_mode().unwrap();
  for option in options[start..end].iter() {
    let mut output: String;
    match option.option_type {
      InteractiveOptionType::Header => {
        output = format!(" {}", option.text);
        if !stop {
          real_index -= 1;
        }
      },
      InteractiveOptionType::Button => {
        if index == selected_index.0 {
          output = format!("   [ {} ]", option.text.clone().underlined().bold());
          stop = true;
        } else {
          output = format!("   [ {} ]", option.text);
        }
      },
      InteractiveOptionType::TextInput => {
        let input = inputs.get(index).unwrap();
        let mut text = if !input.is_empty() {
          let prefix = if selected_index.1 != 0 {
            input.take_chars(0, selected_index.1 - 1)
          } else {
            String::new()
          };
          format!(
            "{}{}{}",
            prefix,
            input
              .chars()
              .nth(selected_index.1)
              .unwrap_or(' ')
              .underlined(),
            input.take_chars(selected_index.1 + 1, input.chars().count())
          )
        } else {
          String::new()
        };
        if text.is_empty() {
          text = format!("{}", " ".underlined());
        }
        if index == selected_index.0 {
          output = format!(
            "   [ {}: {} ]",
            option.text.clone().underlined().bold(),
            text
          );
          stop = true;
        } else {
          output = format!("   [ {}: {} ]", option.text, inputs.get(index).unwrap());
        }
      },
      InteractiveOptionType::Special => {
        if index == selected_index.0 {
          output = format!("   > {} <", option.text.clone().underlined().bold());
          stop = true;
        } else {
          output = format!("   > {} <", option.text);
        }
      },
      InteractiveOptionType::MultiButton => {
        output = String::from("   [");
        for (button_index, button) in option.text.split_terminator(':').enumerate() {
          if selected_index.1 == button_index && index == selected_index.0 {
            output += &format!(" {} ", button.underlined().bold());
            stop = true;
          } else {
            output += &format!(" {} ", button);
            if selected_index.0 != index {
              output += "] ";
              break;
            }
          }
          if button_index == 0 {
            output += "] ";
          } else {
            output += ":";
          }
        }
        output.pop();
      },
      InteractiveOptionType::ListButtons => {
        output = String::from("   [");
        for (button_index, button) in option.text.split_terminator(':').enumerate() {
          if selected_index.1 == button_index && index == selected_index.0 {
            output += &format!(" {} ", button.underlined().bold());
            stop = true;
          } else {
            output += &format!(" {} ", button);
            if button_index == 0 && selected_index.0 == index {
              output.pop();
            }
            if selected_index.0 != index {
              output += " ";
              break;
            }
          }
          if button_index == 0 {
            output += ":";
          } else {
            output += "|";
          }
        }
        output.pop();
        output += "]";
      },
      InteractiveOptionType::Button5s => {
        if index == selected_index.0 {
          output = format!(
            "   [ {}{} ]",
            option
              .text
              .clone()
              .take_chars(0, selected_index.1)
              .underlined()
              .bold()
              .on_white()
              .black(),
            option
              .text
              .clone()
              .take_chars(selected_index.1 + 1, option.text.chars().count())
              .underlined()
              .bold()
          );
          stop = true;
        } else {
          output = format!("   [ {} ]", option.text);
        }
      },
    }
    if index == options.len() - 1 {
      print!("{}", output);
    } else {
      println!("{}", output);
    }
    if !stop {
      real_index += 1;
    }
    index += 1;
  }
  if start != 0 {
    real_index += start as i16 - 1;
  }
  if end < options.len() {
    println!("{}[⇣]", " ".repeat(terminal_width - 5));
  }
  stdout().flush().expect("Failed to flush stdout");
  enable_raw_mode().unwrap();
  (real_index as usize, selected_index.1)
}

pub fn interactive_select(
  options: Vec<InteractiveOption>,
) -> ((usize, usize), Option<String>, InteractiveOptionType) {
  let terminal_height = terminal::size().unwrap().1 as usize - 3;
  enable_raw_mode().unwrap();
  let mut stdout = io::stdout();
  execute!(
    stdout,
    Hide,
    MoveToColumn(0),
    Clear(ClearType::FromCursorDown)
  )
  .unwrap();
  let mut selection = (0, 0);
  loop {
    match options[selection.0].option_type {
      InteractiveOptionType::Header => {
        selection.0 += 1;
      },
      InteractiveOptionType::ListButtons => {
        selection.1 = 1;
        break;
      },
      _ => break,
    }
  }
  let mut inputs: Vec<String> = vec![];
  for option in options.clone() {
    match option.option_type {
      InteractiveOptionType::Special => inputs.append(&mut vec![option.text]),
      _ => inputs.append(&mut vec![String::new()]),
    }
  }
  if options.len() > terminal_height {
    execute!(stdout, EnterAlternateScreen, DisableLineWrap, Hide).unwrap();
  }
  let mut corrected_selection = display_options(&options, selection, inputs.clone());
  let mut update = false;
  loop {
    if crossterm::event::poll(Duration::from_millis(250)).unwrap() {
      if let Event::Key(KeyEvent {
        code,
        modifiers,
        kind: KeyEventKind::Press,
        ..
      }) = crossterm::event::read().unwrap()
      {
        match code {
          KeyCode::Up => {
            if selection.0 >= 1 {
              selection.0 -= 1;
            } else {
              selection.0 = options.len() - 1;
            }
            loop {
              match options[selection.0].option_type {
                InteractiveOptionType::Header => {
                  if selection.0 == 0 {
                    selection.0 = options.len() - 1;
                  } else {
                    selection.0 -= 1;
                  }
                },
                InteractiveOptionType::ListButtons => {
                  selection.1 = 1;
                  break;
                },
                InteractiveOptionType::TextInput | InteractiveOptionType::Button5s => {
                  selection.1 = 0;
                  break;
                },
                _ => break,
              }
            }
          },
          KeyCode::Down => {
            if selection.0 < options.len() - 1 {
              selection.0 += 1;
            } else {
              selection.0 = 0;
            }
            loop {
              match options[selection.0].option_type {
                InteractiveOptionType::Header => {
                  selection.0 += 1;
                },
                InteractiveOptionType::ListButtons => {
                  selection.1 = 1;
                  break;
                },
                InteractiveOptionType::TextInput | InteractiveOptionType::Button5s => {
                  selection.1 = 0;
                  break;
                },
                _ => break,
              }
            }
          },
          KeyCode::Left => {
            if options[selection.0].option_type == InteractiveOptionType::MultiButton {
              if selection.1 >= 1 {
                selection.1 -= 1;
              } else {
                selection.1 = options[selection.0].text.split_terminator(':').count() - 1;
              }
            } else if options[selection.0].option_type == InteractiveOptionType::ListButtons {
              if selection.1 >= 2 {
                selection.1 -= 1;
              } else {
                selection.1 = options[selection.0].text.split_terminator(':').count() - 1;
              }
            } else if options[selection.0].option_type == InteractiveOptionType::TextInput
              && !inputs[selection.0].is_empty()
            {
              if selection.1 >= 1 {
                selection.1 -= 1;
              } else {
                selection.1 = inputs[selection.0].chars().count();
              }
            }
          },
          KeyCode::Right => {
            if options[selection.0].option_type == InteractiveOptionType::TextInput {
              if !inputs[selection.0].is_empty() {
                if selection.1 < inputs[selection.0].chars().count() {
                  selection.1 += 1;
                } else {
                  selection.1 = 0;
                }
              }
            } else if selection.1 < options[selection.0].text.split_terminator(':').count() - 1 {
              selection.1 += 1;
            } else if options[selection.0].option_type == InteractiveOptionType::MultiButton {
              selection.1 = 0;
            } else if options[selection.0].option_type == InteractiveOptionType::ListButtons {
              selection.1 = 1;
            }
          },
          KeyCode::Enter => break,
          _ => {
            if KeyCode::Char('c') == code && modifiers == KeyModifiers::CONTROL {
              execute!(stdout, Show).unwrap();
              disable_raw_mode().unwrap();
              exit(1);
            } else if let KeyCode::Char(ch) = code {
              if (ch.is_ascii() || ch.is_alphabetic())
                && terminal::size().unwrap().0 as usize
                  > inputs[selection.0].chars().count() + options[selection.0].text.len() + 11
              {
                inputs[selection.0].insert_char(selection.1, ch);
                selection.1 += 1;
              }
            } else if code == KeyCode::Backspace {
              if !inputs[selection.0].is_empty() && selection.1 != 0 {
                inputs[selection.0].remove_char(selection.1 - 1);
                selection.1 -= 1;
              }
              if inputs[selection.0].is_empty() {
                selection.1 = 0;
              }
            }
          },
        };
        update = true;
      } else {
        continue;
      }
    }
    if options[selection.0].option_type == InteractiveOptionType::Button5s {
      let old: usize = selection.1;
      selection.1 +=
        (options[selection.0].text.len() as f64 / 5.0 / 2.0_f64.powi(2)).round() as usize;
      if selection.1 != old {
        update = true;
      }
      if selection.1 > options[selection.0].text.len() {
        selection.1 = 0;
        break;
      }
    }
    if update {
      if options.len() != 1 {
        execute!(
          stdout,
          MoveToPreviousLine((options.len() - 1) as u16),
          Clear(ClearType::FromCursorDown)
        )
        .unwrap();
      } else {
        execute!(stdout, MoveToColumn(0)).unwrap();
      }
      corrected_selection = display_options(&options, selection, inputs.clone());
      update = false;
    }
  }
  if options.len() == 1 {
    execute!(
      stdout,
      MoveToColumn(0),
      Clear(ClearType::UntilNewLine),
      Show
    )
    .unwrap();
  } else {
    execute!(
      stdout,
      MoveToPreviousLine((options.len() - 1) as u16),
      Clear(ClearType::FromCursorDown),
      Show
    )
    .unwrap();
  }
  if options.len() > terminal_height {
    execute!(stdout, LeaveAlternateScreen, EnableLineWrap, Show).unwrap();
  }
  disable_raw_mode().unwrap();
  if options[selection.0].option_type == InteractiveOptionType::Button
    || options[selection.0].option_type == InteractiveOptionType::Button5s
  {
    (
      corrected_selection,
      Some(options[selection.0].text.clone()),
      InteractiveOptionType::Button,
    )
  } else {
    (
      corrected_selection,
      Some(inputs[selection.0].clone()),
      options[selection.0].option_type.clone(),
    )
  }
}

pub fn getch(allowed: &str) -> char {
  adv_getch(allowed, false, None, "").unwrap()
}

pub fn clear_stdin() {
  enable_raw_mode().unwrap();
  loop {
    if poll(Duration::from_millis(100)).unwrap() {
      if read().is_ok() {
        continue;
      }
    } else {
      disable_raw_mode().unwrap();
      return;
    }
  }
}

pub fn adv_getch(
  allowed: &str,
  any_key: bool,
  timeout_secs: Option<u64>,
  message: &str,
) -> Option<char> {
  let mut stdout = stdout();
  let mut timer = timeout_secs.map(|seconds| seconds * 2);

  if let Some(time) = timer {
    print!("\n{} [{}]: ", message, time / 2);
  } else {
    print!("\n{}: ", message);
  }
  stdout.flush().expect("Failed to flush stdout");

  enable_raw_mode().unwrap();
  execute!(stdout, EnableBlinking, Show).unwrap();

  loop {
    if poll(Duration::from_millis(500)).unwrap() {
      if let Ok(Event::Key(KeyEvent {
        code,
        modifiers,
        state: _,
        kind: KeyEventKind::Press,
      })) = read()
      {
        if modifiers == KeyModifiers::NONE && !any_key {
          for ch in allowed.chars() {
            if code == KeyCode::Char(ch) {
              disable_raw_mode().unwrap();
              println!("{}", ch);
              return Some(ch);
            } else if ch == '\n' && code == KeyCode::Enter {
              disable_raw_mode().unwrap();
              println!();
              return Some(ch);
            }
          }
          if code == KeyCode::Up
            || code == KeyCode::Left
            || code == KeyCode::Right
            || code == KeyCode::Down
          {
            continue;
          }
          writeln!(stdout).unwrap();
          execute!(stdout, MoveToNextLine(1)).unwrap();
          writeln!(stdout, "{}", INVALID_INPUT).unwrap();
          execute!(stdout, MoveToNextLine(1)).unwrap();
          if let Some(time) = timer {
            write!(stdout, "{} [{}]: ", message, time / 2).unwrap();
          } else {
            write!(stdout, "{}: ", message).unwrap();
          }
          stdout.flush().expect("Failed to flush stdout");
        } else if modifiers == KeyModifiers::CONTROL && code == KeyCode::Char('c') {
          write!(stdout, "^C").unwrap();
          disable_raw_mode().unwrap();
          exit(1);
        } else if any_key {
          disable_raw_mode().unwrap();
          println!();
          return Some('_'); // this is a smiley
        }
      }
    } else if let Some(time) = timer {
      timer = Some(time - 1);
      execute!(stdout, MoveToColumn(0)).unwrap();
      execute!(stdout, Clear(ClearType::CurrentLine)).unwrap();
      if timer == Some(0) {
        disable_raw_mode().unwrap();
        return None;
      } else {
        write!(stdout, "{} [{}]: ", message, time / 2).unwrap();
        stdout.flush().expect("Failed to flush stdout");
      }
    }
  }
}

pub fn take_string_input(allowed: Vec<String>) -> String {
  loop {
    print!(": ");
    let mut input = String::new();
    stdout().flush().expect("Failed to flush stdout");
    stdin().read_line(&mut input).expect("Failed to read line");

    let input = input.trim().to_string();
    if allowed.contains(&input) || allowed.is_empty() {
      return input;
    } else if input.is_empty() {
      return allowed.first().unwrap().to_string();
    } else {
      println!("{}", INVALID_INPUT);
    }
  }
}

pub fn hidden_string_input(mask: Option<char>) -> String {
  let mut stdout = stdout();
  stdout.flush().expect("Failed to flush stdout");

  enable_raw_mode().unwrap();
  execute!(stdout, EnableBlinking, Show).unwrap();
  execute!(stdout, SavePosition).unwrap();

  let mut input = String::new();
  loop {
    if poll(Duration::from_millis(500)).unwrap() {
      if let Ok(Event::Key(KeyEvent {
        code,
        modifiers,
        state: _,
        kind: KeyEventKind::Press,
      })) = read()
      {
        if modifiers == KeyModifiers::CONTROL && code == KeyCode::Char('c') {
          write!(stdout, "^C").unwrap();
          disable_raw_mode().unwrap();
          execute!(stdout, RestorePosition, Clear(ClearType::FromCursorDown)).unwrap();
          exit(1);
        } else if code == KeyCode::Enter {
          disable_raw_mode().unwrap();
          execute!(stdout, RestorePosition, Clear(ClearType::FromCursorDown)).unwrap();
          println!("---");
          return input;
        } else if code == KeyCode::Backspace {
          if input.pop().is_some() {
            execute!(std::io::stdout(), MoveLeft(1)).unwrap();
            execute!(std::io::stdout(), Print(" "), MoveLeft(1)).unwrap();
          }
        } else {
          if let KeyCode::Char(ch) = code {
            if let Some(masking_char) = mask {
              write!(stdout, "{}", masking_char).unwrap();
            } else {
              write!(stdout, "{}", ch).unwrap();
            }
            input.push(ch);
          }
          stdout.flush().expect("Failed to flush stdout");
        }
      }
    }
  }
}
