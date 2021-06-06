extern crate termion;
use std::cmp;
use std::convert::{TryFrom, TryInto};
use std::fs::File;
use std::io::{self, stdout, BufRead, BufReader, Write};
use termion::color;
use termion::raw::IntoRawMode;
#[derive(Debug, Copy, Clone)]
pub enum Status {
    Empty,
    Marked,
    Unknown,
}
impl Default for Status {
    fn default() -> Self {
        Status::Unknown
    }
}
#[derive(Debug, Default, Copy, Clone)]
struct Point {
    cell_state: Status,
}

#[derive(Debug, Default, Copy, Clone)]
struct NonoKey {
    value: usize,
    upper_bound: usize,
    lower_bound: usize,
}
#[derive(Default, Clone)]
struct Board {
    grid: Vec<Vec<Point>>,
    h_keys: Vec<Vec<NonoKey>>,
    v_keys: Vec<Vec<NonoKey>>,
    size: Vec<usize>,
}

const CELL: &str = "▉▉▉▉▉";
const UNKNOWN: &str = "?????";

fn main() {
    let mut b: Board = Default::default();
    let mut vkey_max: usize = 0;
    let mut hkey_max: usize = 0;
    let file: String = user_puzzle_choice();

    b.read_nonogram(file).expect("Could not read file");

    if !init(&b) {
        println!("The given board is too large, try a smaller size.");
        return;
    }

    let mut stdout = stdout().into_raw_mode().unwrap();

    b.get_key_dimensions(&mut vkey_max, &mut hkey_max);
    print_keys(&b, hkey_max, vkey_max, &mut stdout);
    find_solution(b, vkey_max, hkey_max, stdout);
}

//user chooses the nonogram they want to solve
fn user_puzzle_choice() -> String {
    println!("Choose a puzzle (1-7): ");
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to parse input.");
    match input.trim().parse::<usize>().unwrap() {
        1 => "./puzzles/nono1.txt".to_string(),
        2 => "./puzzles/nono2.txt".to_string(),
        3 => "./puzzles/nono3.txt".to_string(),
        4 => "./puzzles/nono4.txt".to_string(),
        5 => "./puzzles/nono5.txt".to_string(),
        6 => "./puzzles/nono6.txt".to_string(),
        7 => "./puzzles/nono7.txt".to_string(),
        _ => panic!("Invalid input"),
    }
}

//print the vertical and horizonal keys in the terminal
fn print_keys<W: Write>(b: &Board, hkey_max: usize, vkey_max: usize, stdout: &mut W) {
    write!(stdout, "{}", termion::clear::All).expect("Could not display keys");
    for (i, key_set) in b.v_keys.iter().enumerate() {
        for (space, (j, key)) in key_set.iter().rev().enumerate().enumerate() {
            write!(
                stdout,
                "{}",
                termion::cursor::Goto(
                    (vkey_max + (vkey_max - 1) - j - space).try_into().unwrap(),
                    u16::try_from((i + 1) * 2 + hkey_max - 1).ok().unwrap()
                )
            )
            .expect("Could not display vertical keys");
            println!("{}", key.value);
        }
    }

    for (space, (i, key_set)) in b.h_keys.iter().enumerate().enumerate() {
        for (j, key) in key_set.iter().rev().enumerate() {
            write!(
                stdout,
                "{}",
                termion::cursor::Goto(
                    (i + 1 + vkey_max + vkey_max - 1 + (space * 4))
                        .try_into()
                        .unwrap(),
                    u16::try_from(hkey_max - j).ok().unwrap()
                )
            )
            .expect("Could not display horizontal keys");
            println!("{}", key.value);
        }
    }
}

//check that the terminal is large enough to fit the given puzzle
fn init(board: &Board) -> bool {
    let termsize = termion::terminal_size().ok();
    let termwidth = termsize.map(|(w, _)| w - 2).unwrap();
    let termheight = termsize.map(|(_, h)| h - 2).unwrap();
    if board.v_keys.len() * 2 + 6 > termheight.into() {
        return false;
    }
    if board.h_keys.len() * 2 + 6 > termwidth.into() {
        return false;
    }
    true
}

//solves the given puzzle using deductive methods until no more deductions are possible
//once deductive reasoning cannot be used, make a guess, check whether the guess makes for a valid solution
//if valid, keep using deduction, otherwise, pop from the stack and make another guess
fn find_solution<W: Write>(mut b: Board, vkey_max: usize, hkey_max: usize, mut stdout: W) {
    b.size.swap(0, 1);
    b.solve();
    let mut grid = Vec::new();
    let mut guess = Vec::new();
    grid.push(b);
    guess.push(None);
    while !grid.is_empty() {
        let a_grid = grid.last().unwrap();
        if !a_grid.valid_grid() {
            grid.pop();
            guess.pop();
            continue;
        }
        if a_grid.complete_grid() {
            a_grid.update_board(&mut stdout, &vkey_max, &hkey_max);
            stdout.flush().unwrap();
            return;
        }
        if let Some((_, Status::Marked)) = guess.last().unwrap() {
            grid.pop();
            guess.pop();
            continue;
        }

        let a_guess = guess.last_mut().unwrap();
        if a_guess.is_none() {
            let (i, j) = a_grid.get_unknown_cell();
            *a_guess = Some(((i, j), Status::Empty));
            let mut new_grid = a_grid.clone();
            new_grid.grid[i][j].cell_state = Status::Empty;
            new_grid.solve();
            grid.push(new_grid);
            guess.push(None);
        } else {
            let (i, j) = a_grid.get_unknown_cell();
            *a_guess = Some(((i, j), Status::Marked));
            let mut new_grid = a_grid.clone();
            new_grid.grid[i][j].cell_state = Status::Marked;
            new_grid.solve();
            new_grid.solve();
            grid.push(new_grid);
            guess.push(None);
        }
    }
    println!("No Solution");
}
impl Board {
    //returns true if all the cells in the grid are set to either marked or empty
    fn complete_grid(&self) -> bool {
        for i in 0..self.size[0] {
            for j in 0..self.size[1] {
                if matches!(self.grid[i][j].cell_state, Status::Unknown) {
                    return false;
                }
            }
        }
        true
    }
    
    //read the nonogram in the given file
    pub fn read_nonogram(&mut self, file: String) -> std::io::Result<()> {
        let mut size = Vec::<usize>::new();
        let mut h_keys: Vec<Vec<NonoKey>> = Vec::new();
        let mut v_keys: Vec<Vec<NonoKey>> = Vec::new();
        let file = File::open(file)?;
        let reader = BufReader::new(file);
        for (count, line) in reader.lines().enumerate() {
            if count == 0 {
                let line = line.unwrap();
                let value = line.split(',');
                size = value.map(|s| s.parse().unwrap()).collect();
            } else if count > 0 && count <= size[0] {
                let line = line.unwrap();
                let value = line.split(',');
                let temp_v: Vec<usize> = value.map(|s| s.parse().unwrap()).collect();
                let v: Vec<NonoKey> = (0..temp_v.len())
                    .map(|x| NonoKey {
                        value: temp_v[x],
                        lower_bound: 0,
                        upper_bound: 0,
                    })
                    .collect();

                h_keys.push(v);
            } else {
                let line = line.unwrap();
                let value = line.split(',');
                let temp_v: Vec<usize> = value.map(|s| s.parse().unwrap()).collect();
                let v: Vec<NonoKey> = (0..temp_v.len())
                    .map(|x| NonoKey {
                        value: temp_v[x],
                        lower_bound: 0,
                        upper_bound: 0,
                    })
                    .collect();
                v_keys.push(v);
            }
        }
        let grid = vec![
            vec![
                Point {
                    cell_state: Status::Unknown
                };
                size[0]
            ];
            size[1]
        ];

        self.grid = grid;
        self.h_keys = h_keys;
        self.v_keys = v_keys;
        self.size = size;
        Ok(())
    }
    //returns the maximum number of keys in a set
    fn get_key_dimensions(&self, vkey: &mut usize, hkey: &mut usize) {
        for i in self.v_keys.iter() {
            if i.len() > *vkey {
                *vkey = i.len();
            }
        }
        for i in self.h_keys.iter() {
            if i.len() > *hkey {
                *hkey = i.len();
            }
        }
    }

    //loops until no further deductive steps are possible to make progress on the solution
    fn solve(&mut self) {
        self.determine_bounds_v();
        self.determine_bounds_h();
        let mut progress = true;
        while progress {
            progress = self.definite_within_bounds_v()
                | self.definite_within_bounds_h()
                | self.determine_spaces_between_keys_v()
                | self.determine_spaces_between_keys_h()
                | self.separate_keys_v()
                | self.separate_keys_h()
                | self.tighten_bounds_v()
                | self.tighten_bounds_h()
                | self.complete_groups_v()
                | self.complete_groups_h();
        }
    }

    //figure out the bounds for each key by counting from the edgdes, considering preceding keys
    fn determine_bounds_v(&mut self) -> bool {
        let height = self.v_keys.len();
        let width = self.h_keys.len();
        let mut progress = false;
        for i in 0..height {
            let mut s = 0;
            let len = self.v_keys[i].len();
            for j in 0..len {
                self.v_keys[i][j].upper_bound = s;
                s += self.v_keys[i][j].value + 1; //number of spaces taken up by previous key value + space
                progress = true;
            }
            s = 0;
            for j in (0..len).rev() {
                if width - s >= self.v_keys[i][j].value {
                    self.v_keys[i][j].lower_bound = width - s;
                    s += self.v_keys[i][j].value + 1;
                    progress = true;
                }
            }
        }
        progress
    }

    fn determine_bounds_h(&mut self) -> bool {
        let height = self.v_keys.len();
        let width = self.h_keys.len();
        let mut progress = false;
        for i in 0..width {
            let mut s = 0;
            let len = self.h_keys[i].len();
            for j in 0..len {
                //from front to back
                self.h_keys[i][j].upper_bound = s;
                s += self.h_keys[i][j].value + 1; //number of spaces taken up by previous key value + space
                progress = true;
            }
            s = 0;
            for j in (0..len).rev() {
                if height - s >= self.h_keys[i][j].value {
                    self.h_keys[i][j].lower_bound = height - s;
                    s += self.h_keys[i][j].value + 1;
                    progress = true;
                }
            }
        }
        progress
    }

    //Set cells to marked if a given bound has definite marked cells within it.
    //If the bound indicates that the entire key has been marked, mark the bounds as empty. 
    fn definite_within_bounds_v(&mut self) -> bool {
        let height = self.v_keys.len();
        let width = self.h_keys.len();
        let mut progress = false;
        for i in 0..height {
            let len = self.v_keys[i].len();
            for j in 0..len {
                let def_upper = self.v_keys[i][j].lower_bound - self.v_keys[i][j].value;
                let def_lower = self.v_keys[i][j].upper_bound + self.v_keys[i][j].value;

                if def_lower >= def_upper {
                    for n in def_upper..def_lower {
                        if n >= width {
                            continue;
                        }
                        if matches!(self.grid[i][n].cell_state, Status::Unknown) {
                            self.grid[i][n].cell_state = Status::Marked;
                            progress = true;
                        }
                    }
                    if def_lower - def_upper == self.v_keys[i][j].value {
                        if def_lower < width - 1
                            && matches!(self.grid[i][def_lower].cell_state, Status::Unknown)
                        {
                            self.grid[i][def_lower].cell_state = Status::Empty;
                            progress = true;
                        }
                        if def_upper > 0
                            && matches!(self.grid[i][def_upper - 1].cell_state, Status::Unknown)
                        {
                            self.grid[i][def_upper - 1].cell_state = Status::Empty;
                            progress = true;
                        }
                    }
                }
            }
        }
        progress
    }
    fn definite_within_bounds_h(&mut self) -> bool {
        let height = self.v_keys.len();
        let width = self.h_keys.len();
        let mut progress = false;
        for i in 0..width {
            let len = self.h_keys[i].len();
            for j in 0..len {
                if self.h_keys[i][j].lower_bound < self.h_keys[i][j].value {
                    continue;
                }
                let def_upper = self.h_keys[i][j].lower_bound - self.h_keys[i][j].value;
                let def_lower = self.h_keys[i][j].upper_bound + self.h_keys[i][j].value;
                if def_lower > def_upper {
                    for n in def_upper..def_lower {
                        if n >= height {
                            continue;
                        }
                        if matches!(self.grid[n][i].cell_state, Status::Unknown) {
                            self.grid[n][i].cell_state = Status::Marked;
                            progress = true;
                        }
                    }
                    if def_lower - def_upper == self.h_keys[i][j].value {
                        if def_lower < height - 1
                            && matches!(self.grid[def_lower][i].cell_state, Status::Unknown)
                        {
                            self.grid[def_lower][i].cell_state = Status::Empty;
                            progress = true;
                        }
                        if def_upper > 0
                            && matches!(self.grid[def_upper - 1][i].cell_state, Status::Unknown)
                        {
                            self.grid[def_upper - 1][i].cell_state = Status::Empty;
                            progress = true;
                        }
                    }
                }
            }
        }
        progress
    }

    //Zone key bounds further according to known empty cells
    fn separate_keys_v(&mut self) -> bool {
        let height = self.v_keys.len();
        let mut progress = false;
        for i in 0..height {
            let len = self.v_keys[i].len();
            for j in 0..len {
                let mut key = &mut self.v_keys[i][j];
                for n in key.upper_bound..key.lower_bound {
                    match self.grid[i][n].cell_state {
                        Status::Empty => {
                            if n > key.lower_bound {
                                continue;
                            }
                            let upper_space = n - key.upper_bound;
                            let lower_space = key.lower_bound - 1 - n;

                            if upper_space < key.value {
                                key.upper_bound = n + 1;
                                progress = true;
                            }
                            if lower_space < key.value && n >= key.value {
                                key.lower_bound = n;
                                progress = true;
                            }
                        }
                        _ => continue,
                    }
                }
            }
        }
        progress
    }
    fn separate_keys_h(&mut self) -> bool {
        let width = self.h_keys.len();
        let mut progress = false;
        for i in 0..width {
            let len = self.h_keys[i].len();
            for j in 0..len {
                let key = &mut self.h_keys[i][j];
                for n in key.upper_bound..key.lower_bound {
                    match self.grid[n][i].cell_state {
                        Status::Empty => {
                            if n > key.lower_bound {
                                continue;
                            }
                            let upper_space = n - key.upper_bound;
                            let lower_space = key.lower_bound - n - 1;

                            if upper_space < key.value {
                                key.upper_bound = n + 1;
                                progress = true;
                            }
                            if lower_space < key.value && n >= key.value {
                                key.lower_bound = n;
                                progress = true;
                            }
                        }
                        _ => continue,
                    }
                }
            }
        }
        progress
    }
    
    //Marks spaces between keys in a key set if their bounds don't intersect
    fn determine_spaces_between_keys_v(&mut self) -> bool {
        let height = self.v_keys.len();
        let mut progress = false;
        for i in 0..height {
            let len = self.v_keys[i].len();
            let mut prev_lower_bound = self.v_keys[i][0].lower_bound;
            for j in 1..len {
                if self.v_keys[i][j].upper_bound > prev_lower_bound {
                    for n in prev_lower_bound..self.v_keys[i][j].upper_bound {
                        if matches!(self.grid[i][n].cell_state, Status::Unknown) {
                            self.grid[i][n].cell_state = Status::Empty;
                            progress = true;
                        }
                    }
                }
                prev_lower_bound = self.v_keys[i][j].lower_bound;
            }
        }
        progress
    }
    fn determine_spaces_between_keys_h(&mut self) -> bool {
        let width = self.h_keys.len();
        let mut progress = false;
        for i in 0..width {
            let len = self.h_keys[i].len();
            let mut prev_lower_bound = self.h_keys[i][0].lower_bound;
            for j in 1..len {
                if self.h_keys[i][j].upper_bound > prev_lower_bound {
                    for n in prev_lower_bound..self.h_keys[i][j].upper_bound {
                        if matches!(self.grid[n][i].cell_state, Status::Unknown) {
                            self.grid[n][i].cell_state = Status::Empty;
                            progress = true;
                        }
                    }
                }
                prev_lower_bound = self.h_keys[i][j].lower_bound;
            }
        }
        progress
    }

    //Further reduce key bounds if it contains a marked cell that is exclusive to the given bound
    fn tighten_bounds_v(&mut self) -> bool {
        let height = self.v_keys.len();
        let mut progress = false;
        for i in 0..height {
            let len = self.v_keys[i].len();
            let mut prev_bound;
            let mut next_bound;
            for j in 0..len {
                if j < len - 1 {
                    next_bound = self.v_keys[i][j + 1].upper_bound;
                } else {
                    next_bound = self.v_keys[i][j].lower_bound;
                }

                if j > 0 {
                    prev_bound = self.v_keys[i][j - 1].lower_bound;
                } else {
                    prev_bound = self.v_keys[i][j].upper_bound;
                }

                let key = &mut self.v_keys[i][j];
                let min_def = cmp::min(key.lower_bound, prev_bound);
                let max_def = cmp::max(key.upper_bound, next_bound);
                if min_def >= max_def {
                    continue;
                }
                if min_def < max_def {
                    for n in min_def..max_def {
                        if matches!(self.grid[i][n].cell_state, Status::Marked) {
                            let mut upper = 0;
                            if n >= key.value {
                                upper = n - key.value;
                            }
                            let lower = n + key.value;
                            if key.upper_bound < upper {
                                key.upper_bound = upper;
                                progress = true;
                            }
                            if key.lower_bound > lower && lower >= key.value {
                                key.lower_bound = lower;
                                progress = true;
                            }
                        }
                    }
                }
            }
        }
        progress
    }
    fn tighten_bounds_h(&mut self) -> bool {
        let width = self.h_keys.len();
        let mut progress = false;
        for i in 0..width {
            let len = self.h_keys[i].len();
            let mut prev_bound;
            let mut next_bound;
            for j in 0..len {
                if j < len - 1 {
                    next_bound = self.h_keys[i][j + 1].upper_bound;
                } else {
                    next_bound = self.h_keys[i][j].lower_bound;
                }

                if j > 0 {
                    prev_bound = self.h_keys[i][j - 1].lower_bound;
                } else {
                    prev_bound = self.h_keys[i][j].upper_bound;
                }

                let key = &mut self.h_keys[i][j];
                let min_def = cmp::min(key.lower_bound, prev_bound);
                let max_def = cmp::max(key.upper_bound, next_bound);
                if min_def >= max_def {
                    continue;
                }

                if max_def > min_def {
                    for n in min_def..max_def {
                        if matches!(self.grid[n][i].cell_state, Status::Marked) {
                            let mut upper = 0;
                            if n >= key.value {
                                upper = n - key.value;
                            }
                            let lower = n + key.value;
                            if key.upper_bound < upper {
                                key.upper_bound = upper;
                                progress = true;
                            }
                            if key.lower_bound > lower && lower >= key.value {
                                key.lower_bound = lower;
                                progress = true;
                            }
                        }
                    }
                }
            }
        }
        progress
    }

    //Mark the remaining cells in the row/cell if all the keys in the set have been determined
    fn complete_groups_v(&mut self) -> bool {
        let height = self.v_keys.len();
        let width = self.h_keys.len();
        let mut progress = false;
        for i in 0..height {
            let len = self.v_keys[i].len();
            let mut complete = 0;
            let mut cell_state = 0;
            for j in 0..width {
                match self.grid[i][j].cell_state {
                    Status::Marked => {
                        if cell_state == 0 {
                            cell_state = 1;
                        }
                    }
                    Status::Empty => {
                        if cell_state == 1 {
                            complete += 1;
                        }
                        cell_state = 0;
                    }
                    Status::Unknown => cell_state = 2,
                }
            }
            if complete == len {
                for n in 0..width {
                    if matches!(self.grid[i][n].cell_state, Status::Unknown) {
                        progress = true;
                        self.grid[i][n].cell_state = Status::Empty;
                    }
                }
            }
        }
        progress
    }
    fn complete_groups_h(&mut self) -> bool {
        let height = self.v_keys.len();
        let width = self.h_keys.len();
        let mut progress = false;
        for i in 0..width {
            let mut complete = 0;
            let mut cell_state = 0;
            for j in 0..height {
                match self.grid[j][i].cell_state {
                    Status::Marked => {
                        if cell_state == 0 {
                            cell_state = 1;
                        }
                    }
                    Status::Empty => {
                        if cell_state == 1 {
                            complete += 1;
                        }
                        cell_state = 0;
                    }
                    Status::Unknown => cell_state = 2,
                }
            }
            if complete == self.h_keys[i].len() {
                for n in 0..height {
                    if matches!(self.grid[n][i].cell_state, Status::Unknown) {
                        progress = true;
                        self.grid[n][i].cell_state = Status::Empty;
                    }
                }
            }
        }
        progress
    }
    
    //Return true if the pattern in keys matches the pattern in compare_to
    fn compare_keys_start(&self, keys: &[usize], compare_to: &[NonoKey]) -> bool {
        let len = keys.len();
        if keys.len() > compare_to.len() {
            return false;
        }
        
        for i in 0..len {
            if keys[i] != compare_to[i].value {
                return false;
            }
        }
        true
    }
    //Return true if both vectors match each other entirely 
    fn compare_keys_whole(&self, keys: &[usize], compare_to: &[NonoKey]) -> bool {
        if keys.len() != compare_to.len() {
            return false;
        }
        let len = keys.len();
        for i in 0..len {
            if keys[i] != compare_to[i].value {
                return false;
            }
        }
        true
    }

    //Return false if a row/column consists of cells that don't correspond with the appropriate key set 
    fn valid_grid(&self) -> bool {
        let height = self.v_keys.len();
        let width = self.h_keys.len();
        let mut keys = Vec::with_capacity(10);
        for i in 0..height {
            keys.clear();
            let mut group = 0;
            let mut t = false;
            for j in 0..width {
                match self.grid[i][j].cell_state {
                    Status::Marked => {
                        group += 1;
                    }
                    Status::Empty => {
                        if group > 0 {
                            keys.push(group);
                        }
                        group = 0;
                    }
                    Status::Unknown => {
                        t = true;
                        break;
                    }
                }
            }

            if group > 0 && !t {
                keys.push(group);
            }
            if t {
                if !self.compare_keys_start(&keys, &self.v_keys[i]) {
                    return false;
                }
            } else {
                if keys.is_empty() {
                    keys.push(0);
                }
                if !self.compare_keys_whole(&keys, &self.v_keys[i]) {
                    return false;
                }
            }
        }
        for i in 0..width {
            keys.clear();
            let mut group = 0;
            let mut t = false;
            for j in 0..height {
                match self.grid[j][i].cell_state {
                    Status::Marked => group += 1,
                    Status::Empty => {
                        if group > 0 {
                            keys.push(group);
                        }
                        group = 0;
                    }
                    Status::Unknown => {
                        t = true;
                        break;
                    }
                }
            }
            if group > 0 && !t {
                keys.push(group);
            }
            if t {
                if !self.compare_keys_start(&keys, &self.h_keys[i]) {
                    return false;
                }
            } else {
                if keys.is_empty() {
                    keys.push(0);
                }
                if !self.compare_keys_whole(&keys, &self.h_keys[i]) {
                    return false;
                }
            }
        }

        true
    }
    //Finds and returns the first unmarked cell in the grid
    fn get_unknown_cell(&self) -> (usize, usize) {
        let height = self.v_keys.len();
        let width = self.h_keys.len();
        for i in 0..height {
            for j in 0..width {
                if matches!(self.grid[i][j].cell_state, Status::Unknown) {
                    return (i, j);
                }
            }
        }
        println!("cant find unknown");
        (0, 0)
    }

    //Prints the complete grid in the terminal
    fn update_board<W: Write>(&self, stdout: &mut W, vkey_max: &usize, hkey_max: &usize) {
        for (i, cell_set) in self.grid.iter().enumerate() {
            for (j, cell) in cell_set.iter().enumerate() {
                write!(
                    stdout,
                    "{}",
                    termion::cursor::Goto(
                        ((vkey_max + (vkey_max - 1) + 1) + (j * 5))
                            .try_into()
                            .unwrap(),
                        u16::try_from(hkey_max + 1 + (i * 2)).ok().unwrap()
                    )
                )
                .expect("Error updating grid");
                match cell.cell_state {
                    Status::Empty => println!(
                        "{fg}{}{reset_fg}",
                        CELL,
                        fg = color::Fg(color::White),
                        reset_fg = color::Fg(color::Reset)
                    ),
                    Status::Marked => println!(
                        "{fg}{}{reset_fg}",
                        CELL,
                        fg = color::Fg(color::Black),
                        reset_fg = color::Fg(color::Reset)
                    ),
                    Status::Unknown => println!("{}", UNKNOWN),
                }
                write!(
                    stdout,
                    "{}",
                    termion::cursor::Goto(
                        ((vkey_max + (vkey_max - 1) + 1) + (j * 5))
                            .try_into()
                            .unwrap(),
                        u16::try_from(hkey_max + 1 + (i * 2) + 1).ok().unwrap()
                    )
                )
                .expect("Error updating grid");
                match cell.cell_state {
                    Status::Empty => println!(
                        "{fg}{}{reset_fg}",
                        CELL,
                        fg = color::Fg(color::White),
                        reset_fg = color::Fg(color::Reset)
                    ),
                    Status::Marked => println!(
                        "{fg}{}{reset_fg}",
                        CELL,
                        fg = color::Fg(color::Black),
                        reset_fg = color::Fg(color::Reset)
                    ),
                    Status::Unknown => println!("{}", UNKNOWN),
                }
            }
        }
        write!(
            stdout,
            "{}",
            termion::cursor::Goto(1, (self.size[0] * 2 + 5).try_into().unwrap())
        )
        .expect("Error updating board");

        stdout.flush().unwrap();
    }
}
