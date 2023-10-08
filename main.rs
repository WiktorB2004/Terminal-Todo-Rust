use ncurses::*;
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::process;

const REGULAR_PAIR: i16 = 0;
const HIGHLIGHT_PAIR: i16 = 1;

type Id = i32;

#[derive(Default)]
struct UI {
    list_curr: Option<Id>,
    row: i32,
    col: i32,
}

impl UI {
    fn begin(&mut self, row: i32, col: i32) {
        self.row = row;
        self.col = col;
    }

    fn begin_list(&mut self, id: Id) {
        assert!(self.list_curr.is_none(), "Nested lists are not allowed");
        self.list_curr = Some(id);
    }

    fn label(&mut self, text: &str, pair: i16) {
        mv(self.row as i32, self.col as i32);
        attron(COLOR_PAIR(pair));
        addstr(text);
        attroff(COLOR_PAIR(pair));
        self.row += 1;
    }

    fn list_element(&mut self, label: &str, id: Id) {
        let id_curr = self
            .list_curr
            .expect("Not allowed to create list elements outside of lists");

        self.label(label, {
            if id_curr == id {
                HIGHLIGHT_PAIR
            } else {
                REGULAR_PAIR
            }
        });
    }

    fn end_list(&mut self) {
        self.list_curr = None;
    }

    fn end(&mut self) {}
}
#[derive(Debug)]
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
        ui.begin(0, 0);
        match focus {
            Focus::Todo => {
                ui.begin_list(todo_curr);
                ui.label("[TODO] DONE", REGULAR_PAIR);
                ui.label("-------------", REGULAR_PAIR);
                for (index, todo) in todos.iter().enumerate() {
                    ui.list_element(&format!("- [ ] {}", todo), index as i32);
                }
                ui.end_list();
            }
            Focus::Done => {
                ui.label("TODO [DONE]", REGULAR_PAIR);
                ui.label("-------------", REGULAR_PAIR);
                ui.begin_list(done_curr);
                for (index, done) in done.iter().enumerate() {
                    ui.list_element(&format!("- [x] {}", done), index as i32)
                }
                ui.end_list();
            }
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
