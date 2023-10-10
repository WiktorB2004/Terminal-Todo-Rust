use ncurses::*;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::ops::{Add, Mul};
use std::process;
use std::{cmp, env};

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

    fn label(&mut self, text: &str, pair: i16) {
        let layout = self
            .containers
            .last_mut()
            .expect("Trying to render label outside existing layout");

        let pos = layout.available_pos();

        mv(pos.col as i32, pos.row as i32);
        attron(COLOR_PAIR(pair));
        addstr(text);
        attroff(COLOR_PAIR(pair));
        layout.add_widget(Vec2::new(text.len() as i32, 1));
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
            Focus::Todo => return Focus::Done,
            Focus::Done => return Focus::Todo,
        }
    }
}

fn list_up(_list: &Vec<String>, list_curr: &mut i32) {
    if *list_curr > 0 {
        *list_curr -= 1;
    }
}

fn list_down(list: &Vec<String>, list_curr: &mut i32) {
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
    let todo_prefix = "TODO: ";
    let done_prefix = "DONE: ";
    if line.starts_with(todo_prefix) {
        return Some((Focus::Todo, &line[todo_prefix.len()..]));
    }
    if line.starts_with(done_prefix) {
        return Some((Focus::Done, &line[done_prefix.len()..]));
    }
    todo!();
}

fn save_state(todos: &Vec<String>, done: &Vec<String>, file_path: &str) {
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

    let mut todos = Vec::<String>::new();
    let mut done = Vec::<String>::new();
    let mut todo_curr: i32 = 0;
    let mut done_curr: i32 = 0;

    load_state(&mut todos, &mut done, &file_path);

    initscr();
    noecho();
    curs_set(CURSOR_VISIBILITY::CURSOR_INVISIBLE);

    start_color();
    init_pair(REGULAR_PAIR, COLOR_WHITE, COLOR_BLACK);
    init_pair(HIGHLIGHT_PAIR, COLOR_BLACK, COLOR_WHITE);

    refresh();
    let mut quit = false;
    let mut focus = Focus::Todo;

    let mut ui = UI::default();

    while !quit {
        erase();
        ui.begin(Vec2::new(0, 0), ContType::Horz);
        {
            ui.begin_container(ContType::Vert);
            {
                ui.label("TODO", REGULAR_PAIR);
                ui.label("-------------", REGULAR_PAIR);
                for (index, item) in todos.iter().enumerate() {
                    ui.label(
                        &format!("- [ ] {}", item),
                        if index == todo_curr as usize && focus == Focus::Todo {
                            HIGHLIGHT_PAIR
                        } else {
                            REGULAR_PAIR
                        },
                    );
                }
            }
            ui.end_container();

            ui.begin_container(ContType::Vert);
            {
                ui.label("DONE", REGULAR_PAIR);
                ui.label("-------------", REGULAR_PAIR);
                for (index, item) in done.iter().enumerate() {
                    ui.label(
                        &format!("- [x] {}", item),
                        if index == done_curr as usize && focus == Focus::Done {
                            HIGHLIGHT_PAIR
                        } else {
                            REGULAR_PAIR
                        },
                    )
                }
            }
            ui.end_container();
        }

        ui.end();
        refresh();

        let key = getch();
        match key as u8 as char {
            'q' => quit = true,
            'w' => match focus {
                Focus::Todo => list_up(&todos, &mut todo_curr),
                Focus::Done => list_up(&done, &mut done_curr),
            },
            's' => match focus {
                Focus::Todo => list_down(&todos, &mut todo_curr),
                Focus::Done => list_down(&done, &mut done_curr),
            },
            '\n' => match focus {
                Focus::Todo => list_transfer(&mut done, &mut todos, &mut todo_curr),
                Focus::Done => list_transfer(&mut todos, &mut done, &mut done_curr),
            },
            '\t' => {
                focus = focus.toggle();
            }
            _ => {}
        }
    }
    save_state(&todos, &done, &file_path);
    endwin();
}
