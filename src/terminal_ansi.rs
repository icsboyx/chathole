#![allow(dead_code)]

use colored::Colorize;

use crate::defs::Terminal;

pub struct AnsiControlCodes;

impl AnsiControlCodes {
    pub fn get_terminal_size(&self) -> String {
        String::from("\x1B[18t")
    }
    pub fn get_cursor_position(&self) -> String {
        String::from("\x1B[6n")
    }
    pub fn clear_screen(&self) -> String {
        String::from("\x1B[2J")
    }
    pub fn enable_local_echo(&self) -> String {
        String::from("\x1B[12h")
    }
    pub fn disable_local_echo(&self) -> String {
        String::from("\x1B[12l")
    }
    pub fn move_cursor(&self, x: u16, y: u16) -> String {
        format!("\x1B[{};{}H", x, y)
    }
    pub fn move_cursor_up(&self, n: u16) -> String {
        format!("\x1B[{}A", n)
    }
    pub fn move_cursor_down(&self, n: u16) -> String {
        format!("\x1B[{}B", n)
    }
    pub fn new_line(&self) -> String {
        "\r\n".to_string()
    }
    pub fn move_cursor_start_of_line(&self) -> String {
        String::from("\x1B[1000D")
    }
    pub fn clear_all(&self) -> String {
        String::from("\x1B[3J")
    }
    pub fn draw_line(&self, length: usize) -> String {
        "-".repeat(length)
    }
    pub fn draw_box(&self, length: usize) -> String {
        format!(
            "{}\r\n{}\r\n{}",
            "#".repeat(length),
            "#".repeat(length),
            "#".repeat(length)
        )
    }
    pub fn enter_alt_screen(&self) -> String {
        String::from("\x1B[?1049h")
    }
    pub fn exit_alt_screen(&self) -> String {
        String::from("\x1B[?1049l")
    }
    pub fn vt100(&self) -> String {
        String::from("\x1B[?1h")
    }
    pub fn disable_enter(&self) -> String {
        String::from("\x1B[?1049l\x1B[?1l\x1B>")
    }
    pub fn save_cursor(&self) -> String {
        String::from("\x1B[s")
    }
    pub fn restore_cursor(&self) -> String {
        String::from("\x1B[u")
    }
    pub fn clear_line(&self) -> String {
        // String::from("\x1B[2K")
        String::from("\x1B[2K")
    }
    pub fn clear_lines_to_end_of_screen(&self) -> String {
        String::from("\x1B[0J")
    }
    pub fn disable_line_wrap(&self) -> String {
        String::from("\x1B[7l")
    }
    pub fn get_terminal_length(&self) -> String {
        String::from("\x1B[18t")
    }
    pub fn move_cursor_the_end_of_terminal(&self) -> String {
        String::from("\x1B[999;0H")
    }
    pub fn set_terminal_size(&self, rows: u16, cols: u16) -> String {
        format!("\x1B[8;{};{}t", rows, cols)
    }
}

pub fn formatted_terminal(terminal: &mut Terminal) -> String {
    let ac = AnsiControlCodes;
    // let motd = [
    //     "Welcome to the Rust Coded IcsBoyX ChatHole server\r\n",
    //     "This is a work in progress\r\n",
    //     "Please be patient\r\n",
    //     "Type /help for a list of commands\r\n",
    // ]
    // .concat();

    if terminal.chat.len() == 0 {
        return [
            ac.move_cursor(0, 0),
            terminal.get_header().blue().bold().to_string(),
            ac.move_cursor(terminal.get_terminal_lines() as u16 + 3, 0),
            terminal.get_prompt().green().bold().to_string(),
        ]
        .concat();
    }

    [
        ac.save_cursor(),
        ac.move_cursor(2, 0),
        ac.clear_line().repeat(terminal.get_terminal_lines()) + "\r\n",
        ac.move_cursor(2, 0),
        terminal.get_chat().dimmed().to_string(),
        ac.restore_cursor(),
    ]
    .concat()
}

pub fn update_prompt(terminal: &mut Terminal) -> String {
    let ac = AnsiControlCodes;
    [
        ac.move_cursor(terminal.get_terminal_lines() as u16 + 3, 0),
        ac.clear_lines_to_end_of_screen(),
        ac.move_cursor(terminal.get_terminal_lines() as u16 + 3, 0),
        terminal.get_prompt().green().bold().to_string(),
    ]
    .concat()
}

pub fn restore_terminal() -> String {
    let ac = AnsiControlCodes;
    [
        ac.clear_all(),
        ac.clear_screen(),
        ac.exit_alt_screen(),
        ac.move_cursor(0, 0),
    ]
    .concat()
}

pub fn init_terminal() -> String {
    let ac = AnsiControlCodes;
    [
        ac.enter_alt_screen(),
        ac.clear_all(),
        ac.set_terminal_size(20, 80),
        ac.move_cursor(0, 0),
    ]
    .concat()
}
