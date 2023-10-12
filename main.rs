use ncurses::*;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::ops::{Add, Mul};
use std::process;
use std::{cmp, env};

// TODO: Create notifications
// TODO: Refactor the code

const REGULAR_PAIR: i16 = 0;
const HIGHLIGHT_PAIR: i16 = 1;

#[derive(Default, Copy, Clone)]
struct Vec2 {
    row: i32,
    col: i32,
}

impl Vec2 {
    fn new(row: i32, col: i32) -> Self {
        Self { row, col }
    }
}

impl Add for Vec2 {
    type Output = Vec2;

    fn add(self, rhs: Vec2) -> Vec2 {
        Vec2 {
            row: self.row + rhs.row,
            col: self.col + rhs.col,
        }
    }
}

impl Mul for Vec2 {
    type Output = Vec2;

    fn mul(self, rhs: Vec2) -> Vec2 {
        Vec2 {
            row: self.row * rhs.row,
            col: self.col * rhs.col,
        }
    }
}

enum ContType {
    Vert,
    Horz,
}

struct Cont {
    kind: ContType,
    pos: Vec2,
    size: Vec2,
}

impl Cont {
    fn available_pos(&self) -> Vec2 {
        use ContType::*;
        match self.kind {
            Horz => self.pos + self.size * Vec2::new(1, 0),
            Vert => self.pos + self.size * Vec2::new(0, 1),
        }
    }

    fn add_widget(&mut self, size: Vec2) {
        use ContType::*;

        match self.kind {
            Horz => {
                self.size.row += size.row;
                self.size.col = cmp::max(self.size.col, size.col);
            }
            Vert => {
                self.size.row = cmp::max(self.size.row, size.row);
                self.size.col += size.col;
            }
        }
    }
}

#[derive(Default)]
struct UI {
    containers: Vec<Cont>,
}

impl UI {
    fn begin(&mut self, pos: Vec2, kind: ContType) {
        assert!(self.containers.is_empty());
        self.containers.push(Cont {
            kind,
            pos,
            size: Vec2::new(0, 0),
        })
    }

    fn begin_container(&mut self, kind: ContType) {
        let layout = self
            .containers
            .last()
            .expect("Cant create container outside of UI::begin() and UI::end()");
        let pos = layout.available_pos();
        self.containers.push(Cont {
            kind,
            pos,
            size: Vec2::new(0, 0),
        })
    }

    fn end_container(&mut self) {
        let layout = self
            .containers
            .pop()
            .expect("Unbalanced UI::begin_layout and UI:end_layout calls");

        self.containers
            .last_mut()
            .expect("Unbalanced UI::begin_layout and UI:end_layout calls")
            .add_widget(layout.size)
    }

    // TODO: UI::edit_item buffer do not scroll after it exceed available space
    fn edit_item(
        &mut self,
        buffer: &mut String,
        cursor: &mut usize,
        key_current: &mut Option<i32>,
        width: i32,
    ) {
        let layout = self
            .containers
            .last_mut()
            .expect("Trying to edit field outside existing layout");

        let pos = layout.available_pos();

        {
            if let Some(key) = key_current.take() {
                match key {
                    32..=126 => {
                        if *cursor >= buffer.len() {
                            buffer.push(key as u8 as char)
                        } else {
                            buffer.insert(*cursor, key as u8 as char)
                        }
                        *cursor += 1;
                    }
                    constants::KEY_LEFT => {
                        if *cursor > 0 {
                            *cursor -= 1;
                        }
                    }
                    constants::KEY_RIGHT => {
                        if *cursor < buffer.len() {
                            *cursor += 1;
                        }
                    }
                    constants::KEY_BACKSPACE => {
                        if *cursor > 0 {
                            *cursor -= 1;
                            if *cursor < buffer.len() {
                                buffer.remove(*cursor);
                            }
                        }
                    }
                    constants::KEY_DC => {
                        if *cursor < buffer.len() {
                            buffer.remove(*cursor);
                        }
                    }
                    _ => {
                        *key_current = Some(key);
                    }
                }
            }

            // BUFFER
            mv(pos.col, pos.row);
            attron(COLOR_PAIR(REGULAR_PAIR));
            addstr(&buffer);
            attroff(COLOR_PAIR(REGULAR_PAIR));
            layout.add_widget(Vec2::new(width, 1));
        }
        // CURSOR
        {
            if *cursor > buffer.len() {
                *cursor = buffer.len();
            }

            mv(pos.col, pos.row + *cursor as i32);
            attron(COLOR_PAIR(HIGHLIGHT_PAIR));
            addstr(buffer.get(*cursor..=*cursor).unwrap_or(" "));
            attroff(COLOR_PAIR(HIGHLIGHT_PAIR));
        }
    }

    fn label_fixed_width(&mut self, text: &str, pair: i16, width: i32) {
        let layout = self
            .containers
            .last_mut()
            .expect("Trying to render label outside existing layout");

        let pos = layout.available_pos();

        mv(pos.col, pos.row);
        attron(COLOR_PAIR(pair));
        addstr(text);
        attroff(COLOR_PAIR(pair));
        layout.add_widget(Vec2::new(width, 1));
    }

    #[allow(dead_code)]
    fn label(&mut self, text: &str, pair: i16) {
        self.label_fixed_width(text, pair, text.len() as i32)
    }

    fn end(&mut self) {
        self.containers
            .pop()
            .expect("Unbalanced UI::begin and UI:end calls");
    }
}
#[derive(Debug, PartialEq)]
enum Focus {
    Todo,
    Done,
}

impl Focus {
    fn toggle(&self) -> Self {
        match self {
            Focus::Todo => Focus::Done,
            Focus::Done => Focus::Todo,
        }
    }
}

fn list_up(_list: &[String], list_curr: &mut i32) {
    if *list_curr > 0 {
        *list_curr -= 1;
    }
}

fn list_down(list: &[String], list_curr: &mut i32) {
    if *list_curr + 1 < list.len() as i32 {
        *list_curr += 1;
    }
}

fn list_transfer(list_dest: &mut Vec<String>, list_src: &mut Vec<String>, list_src_curr: &mut i32) {
    if list_src_curr < &mut (list_src.len() as i32) && *list_src_curr >= 0 {
        list_dest.push(list_src.remove(*list_src_curr as usize));
        *list_src_curr -= 1;
    }
}

fn parse_item(line: &str) -> Option<(Focus, &str)> {
    let todo_item = line
        .strip_prefix("TODO: ")
        .map(|title| (Focus::Todo, title));
    let done_item = line
        .strip_prefix("DONE: ")
        .map(|title| (Focus::Done, title));
    todo_item.or(done_item)
}

fn list_drag_up(list: &mut [String], list_curr: &mut i32) {
    if *list_curr > 0 {
        list.swap(*list_curr as usize, (*list_curr - 1) as usize);
        *list_curr -= 1;
    }
}

fn list_drag_down(list: &mut [String], list_curr: &mut i32) {
    if *list_curr + 1 < list.len() as i32 {
        list.swap(*list_curr as usize, (*list_curr + 1) as usize);
        *list_curr += 1;
    }
}

fn save_state(todos: &[String], done: &[String], file_path: &str) {
    let mut file = File::create(file_path).unwrap();
    for todo in todos.iter() {
        writeln!(file, "TODO: {}", todo).unwrap();
    }
    for done in done.iter() {
        writeln!(file, "DONE: {}", done).unwrap();
    }
}

fn load_state(todos: &mut Vec<String>, done: &mut Vec<String>, file_path: &str) {
    let file = File::open(file_path).unwrap();
    for (index, line) in BufReader::new(file).lines().enumerate() {
        match parse_item(&line.unwrap()) {
            Some((Focus::Todo, title)) => todos.push(title.to_string()),
            Some((Focus::Done, title)) => done.push(title.to_string()),
            None => {
                eprint!("{}:{} ERROR: ill-formed item line", file_path, index + 1);
                process::exit(1)
            }
        }
    }
}

fn main() {
    let mut args = env::args();
    args.next().unwrap();

    let file_path = {
        match args.next() {
            Some(file_path) => file_path,
            None => {
                eprintln!("Usage: main.rs <file-path>");
                eprintln!("ERROR: File path is not provided");
                process::exit(1);
            }
        }
    };

    let mut notification = String::new();
    let mut todos = Vec::<String>::new();
    let mut done = Vec::<String>::new();
    let mut todo_curr: i32 = 0;
    let mut done_curr: i32 = 0;

    load_state(&mut todos, &mut done, &file_path);

    initscr();
    noecho();
    keypad(stdscr(), true);
    timeout(16);
    curs_set(CURSOR_VISIBILITY::CURSOR_INVISIBLE);

    start_color();
    init_pair(REGULAR_PAIR, COLOR_WHITE, COLOR_BLACK);
    init_pair(HIGHLIGHT_PAIR, COLOR_BLACK, COLOR_WHITE);

    refresh();
    let mut quit = false;
    let mut focus = Focus::Todo;
    let mut editing = false;
    let mut editing_cursor = 0;

    let mut ui = UI::default();
    let mut key_curr = None;

    while !quit {
        erase();

        let mut x = 0;
        let mut y = 0;
        getmaxyx(stdscr(), &mut y, &mut x);

        ui.begin(Vec2::new(0, 0), ContType::Vert);
        {
            ui.label_fixed_width(&notification, REGULAR_PAIR, x);
            ui.label_fixed_width(" ", REGULAR_PAIR, x);

            // TODO: items do not scroll after exceceding available space
            ui.begin_container(ContType::Horz);
            {
                ui.begin_container(ContType::Vert);
                {
                    if focus == Focus::Todo {
                        ui.label_fixed_width("TODO", HIGHLIGHT_PAIR, x / 2);
                        for (index, item) in todos.iter_mut().enumerate() {
                            if index == todo_curr as usize {
                                if editing {
                                    ui.edit_item(item, &mut editing_cursor, &mut key_curr, x / 2);
                                    if let Some('\n') = key_curr.take().map(|x| x as u8 as char) {
                                        editing = false;
                                    }
                                } else {
                                    ui.label_fixed_width(
                                        &format!("- [ ] {}", item),
                                        HIGHLIGHT_PAIR,
                                        x / 2,
                                    );
                                    if let Some('e') = key_curr.map(|x| x as u8 as char) {
                                        editing = true;
                                        editing_cursor = item.len();
                                        key_curr = None;
                                    }
                                }
                            } else {
                                ui.label_fixed_width(
                                    &format!("- [ ] {}", item),
                                    REGULAR_PAIR,
                                    x / 2,
                                );
                            }
                        }

                        if let Some(key) = key_curr.take() {
                            match key as u8 as char {
                                'W' => list_drag_up(&mut todos, &mut todo_curr),
                                'w' => list_up(&todos, &mut todo_curr),
                                'S' => list_drag_down(&mut todos, &mut todo_curr),
                                's' => list_down(&todos, &mut todo_curr),
                                'i' => {
                                    todos.insert((todo_curr + 1) as usize, String::new());
                                    todo_curr += 1;
                                    editing_cursor = 0;
                                    editing = true;
                                    key_curr = None;
                                }
                                '\n' => list_transfer(&mut done, &mut todos, &mut todo_curr),
                                '\t' => {
                                    focus = focus.toggle();
                                }
                                _ => {
                                    key_curr = Some(key);
                                }
                            }
                        }
                    } else {
                        ui.label_fixed_width("TODO", REGULAR_PAIR, x / 2);
                        for item in todos.iter() {
                            ui.label_fixed_width(&format!("- [ ] {}", item), REGULAR_PAIR, x / 2)
                        }
                    }
                }
                ui.end_container();

                ui.begin_container(ContType::Vert);
                {
                    if focus == Focus::Done {
                        ui.label_fixed_width("DONE", HIGHLIGHT_PAIR, x / 2);
                        for (index, item) in done.iter_mut().enumerate() {
                            if index == done_curr as usize {
                                if editing {
                                    ui.edit_item(item, &mut editing_cursor, &mut key_curr, x / 2);
                                    if let Some('\n') = key_curr.take().map(|x| x as u8 as char) {
                                        editing = false;
                                    }
                                } else {
                                    ui.label_fixed_width(
                                        &format!("- [x] {}", item),
                                        HIGHLIGHT_PAIR,
                                        x / 2,
                                    );
                                    if let Some('e') = key_curr.map(|x| x as u8 as char) {
                                        editing = true;
                                        editing_cursor = item.len();
                                        key_curr = None;
                                    }
                                }
                            } else {
                                ui.label_fixed_width(
                                    &format!("- [x] {}", item),
                                    REGULAR_PAIR,
                                    x / 2,
                                );
                            }
                        }
                        if let Some(key) = key_curr.take() {
                            match key as u8 as char {
                                'W' => list_drag_up(&mut done, &mut done_curr),
                                'w' => list_up(&done, &mut done_curr),
                                'S' => list_drag_down(&mut done, &mut done_curr),
                                's' => list_down(&done, &mut done_curr),
                                'd' => {
                                    done.remove(done_curr as usize);
                                }
                                '\n' => list_transfer(&mut todos, &mut done, &mut done_curr),
                                '\t' => {
                                    focus = focus.toggle();
                                }
                                _ => {
                                    key_curr = Some(key);
                                }
                            }
                        }
                    } else {
                        ui.label_fixed_width("DONE", REGULAR_PAIR, x / 2);
                        for item in done.iter() {
                            ui.label_fixed_width(&format!("- [x] {}", item), REGULAR_PAIR, x / 2)
                        }
                    }
                }
                ui.end_container();
            }
            ui.end_container();
        }
        ui.end();

        if let Some('q') = key_curr.take().map(|x| x as u8 as char) {
            quit = true;
        }

        refresh();

        let key = getch();
        if key != ERR {
            notification.clear();
            key_curr = Some(key);
        }
    }
    save_state(&todos, &done, &file_path);
    endwin();
}
