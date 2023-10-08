use ncurses::*;

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

fn main() {
    initscr();
    noecho();
    curs_set(CURSOR_VISIBILITY::CURSOR_INVISIBLE);

    start_color();
    init_pair(REGULAR_PAIR, COLOR_WHITE, COLOR_BLACK);
    init_pair(HIGHLIGHT_PAIR, COLOR_BLACK, COLOR_WHITE);

    refresh();
    let mut quit = false;
    let mut todos: Vec<String> = vec![
        "Learn Rust".to_string(),
        "Learn Bash".to_string(),
        "Write TODO app".to_string(),
    ];
    let mut done: Vec<String> = vec![
        "Printing list".to_string(),
        "Have a brekfast".to_string(),
        "Have lunch".to_string(),
    ];
    let mut todo_curr: i32 = 0;
    let mut done_curr: i32 = -1;
    let mut focus = Focus::Todo;

    let mut ui = UI::default();

    while !quit {
        erase();
        ui.begin(0, 0);
        match focus {
            Focus::Todo => {
                ui.label("TODO: ", REGULAR_PAIR);
                ui.begin_list(todo_curr);
                for (index, todo) in todos.iter().enumerate() {
                    ui.list_element(&format!("- [ ] {}", todo), index as i32);
                }
                ui.end_list();
            }
            Focus::Done => {
                ui.label("DONE: ", REGULAR_PAIR);
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
                Focus::Todo => {
                    if todo_curr < todos.len() as i32 {
                        done.push(todos.remove(todo_curr as usize));
                    }
                }
                Focus::Done => {
                    if done_curr < done.len() as i32 {
                        todos.push(done.remove(done_curr as usize));
                    }
                }
            },
            '\t' => {
                focus = focus.toggle();
            }
            _ => {}
        }
    }
    endwin();
}
