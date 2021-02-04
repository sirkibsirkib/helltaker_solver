use std::collections::HashMap;

const ROOM_DIMS: [u8; 2] = [16, 8];
const WORD_BITS: usize = std::mem::size_of::<usize>() * 8;

const INIT_ROOM_BYTE_STR: &[u8] = // newline, thanks
b"
###########|
#### G ####|
####OLO####|
##O#O  # ##|
#O  OOO  K#|
# OOO  OO #|
##@ O  O ##|
###########
";

#[derive(Default, Copy, Clone, Eq, PartialEq, Hash)]
struct CoordSet {
    words: [usize; Self::WORDS],
}

#[derive(derive_new::new, Debug, Copy, Clone, Eq, Hash, PartialEq)]
struct Coord {
    x: u8,
    y: u8,
}

#[derive(Clone, Eq, PartialEq)]
struct RoomImmut {
    walls_at: CoordSet,
    key_at: Option<Coord>,
    lock_at: Option<Coord>,
    goal_at: Coord,
}

#[derive(Clone, Eq, PartialEq, Hash)]
struct RoomMut {
    rocks_at: CoordSet,
    player_at: Coord,
    got_key: bool,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum Direction {
    Up,
    Down,
    Left,
    Right,
}
struct SearchNode {
    pred: Option<(RoomMut, Direction)>,
    steps_left: u32,
}

/////////////////////////

impl Coord {
    fn is_within_bounds(self) -> bool {
        self.x < ROOM_DIMS[0] && self.y < ROOM_DIMS[1]
    }
    fn take_step(self, direction: Direction) -> Option<Self> {
        match direction {
            Direction::Left if self.x == 0 => None,
            Direction::Right if self.x == ROOM_DIMS[0] - 1 => None,
            Direction::Up if self.y == 0 => None,
            Direction::Down if self.y == ROOM_DIMS[1] - 1 => None,
            Direction::Left => Some(Self { x: self.x - 1, y: self.y }),
            Direction::Right => Some(Self { x: self.x + 1, y: self.y }),
            Direction::Up => Some(Self { x: self.x, y: self.y - 1 }),
            Direction::Down => Some(Self { x: self.x, y: self.y + 1 }),
        }
    }
}

fn parse_init(s: &[u8]) -> Option<(RoomImmut, RoomMut)> {
    let mut walls_at = CoordSet::default();
    let mut rocks_at = CoordSet::default();
    let [mut goal_at, mut lock_at, mut key_at, mut player_at] = [None; 4];

    let mut c = Coord::new(0, 0);
    for &byte in s {
        if !c.is_within_bounds() {
            return None;
        }
        match byte {
            b'|' => {
                c.y += 1;
                c.x = 0;
                continue;
            }
            b'#' => walls_at.insert(c),
            b'O' => rocks_at.insert(c),
            b'G' => goal_at = Some(c),
            b'@' => player_at = Some(c),
            b'K' => key_at = Some(c),
            b'L' => lock_at = Some(c),
            b' ' => {}
            _ => continue,
        }
        c.x += 1;
    }
    Some((
        RoomImmut { walls_at, goal_at: goal_at?, key_at, lock_at },
        RoomMut { rocks_at, got_key: false, player_at: player_at? },
    ))
}

impl CoordSet {
    const BITS: usize = ROOM_DIMS[0] as usize * ROOM_DIMS[1] as usize;
    const WORDS: usize = Self::BITS / WORD_BITS;

    fn bit_index_of(coord: Coord) -> usize {
        coord.y as usize * ROOM_DIMS[0] as usize + coord.x as usize
    }
    fn major_minor_of(coord: Coord) -> [usize; 2] {
        let bit_index = Self::bit_index_of(coord);
        [bit_index / WORD_BITS, bit_index % WORD_BITS]
    }
    fn contains(&self, coord: Coord) -> bool {
        let [major, minor] = Self::major_minor_of(coord);
        self.words[major] & (1 << minor) > 0
    }
    fn insert(&mut self, coord: Coord) {
        let [major, minor] = Self::major_minor_of(coord);
        self.words[major] |= 1 << minor;
    }
    fn remove(&mut self, coord: Coord) {
        let [major, minor] = Self::major_minor_of(coord);
        self.words[major] &= !(1 << minor);
    }
}

impl std::iter::FromIterator<Coord> for CoordSet {
    fn from_iter<I: IntoIterator<Item = Coord>>(iter: I) -> Self {
        let mut s = Self::default();
        for i in iter {
            s.insert(i);
        }
        s
    }
}

fn printy(i: &RoomImmut, m: &RoomMut) {
    for y in 0..ROOM_DIMS[1] {
        for x in 0..ROOM_DIMS[0] {
            let c = Coord::new(x, y);
            let byte: char = if m.player_at == c {
                '@'
            } else if i.walls_at.contains(c) {
                '#'
            } else if m.rocks_at.contains(c) {
                'O'
            } else if i.lock_at == Some(c) {
                'L'
            } else if !m.got_key && i.key_at == Some(c) {
                'K'
            } else if i.goal_at == c {
                'G'
            } else {
                ' '
            };
            print!("{}", byte);
        }
        println!();
    }
}

fn main() {
    let (room_immut, init_room_mut) = parse_init(INIT_ROOM_BYTE_STR).unwrap();

    let mut to_visit = vec![init_room_mut.clone()];
    let mut search_nodes: HashMap<RoomMut, SearchNode> =
        maplit::hashmap! { init_room_mut => SearchNode { pred: None, steps_left: 33 }};
    let mut best_solution: Option<(u32, RoomMut)> = None;

    let start = std::time::Instant::now();

    // invariant: solutions elements unique
    // invariant: to_visit have steps_left > 0
    while let Some(room_mut) = to_visit.pop() {
        let mut try_add_room_mut = |search_nodes: &mut HashMap<RoomMut, SearchNode>,
                                    old: &RoomMut,
                                    direction: Direction,
                                    new: RoomMut| {
            let new_steps_left = search_nodes.get(&old).unwrap().steps_left - 1;
            use std::collections::hash_map::Entry;
            let added_new = match search_nodes.entry(new.clone()) {
                Entry::Vacant(v) => {
                    // new configuration
                    v.insert(SearchNode {
                        pred: Some((old.clone(), direction)),
                        steps_left: new_steps_left,
                    });
                    true
                }
                Entry::Occupied(o) => {
                    // already encountered this configuration
                    let o = o.into_mut();
                    if new_steps_left > o.steps_left {
                        o.pred = Some((old.clone(), direction));
                        o.steps_left = new_steps_left;
                        true
                    } else {
                        false
                    }
                }
            };
            if added_new {
                if new.player_at == room_immut.goal_at {
                    if let Some((best_steps_left, best_room_mut)) = best_solution.as_mut() {
                        if *best_steps_left > new_steps_left {
                            *best_steps_left = new_steps_left;
                            *best_room_mut = new.clone();
                        }
                    } else {
                        best_solution = Some((new_steps_left, new.clone()));
                    }
                }
                if new_steps_left > 0 {
                    to_visit.push(new);
                }
            }
        };
        for &direction in [Direction::Up, Direction::Down, Direction::Left, Direction::Right].iter()
        {
            if let Some(step1) = room_mut.player_at.take_step(direction) {
                if !room_immut.walls_at.contains(step1)
                    && !room_mut.rocks_at.contains(step1)
                    && (room_mut.got_key || Some(step1) != room_immut.lock_at)
                {
                    // player can move to `step1`
                    let mut new = room_mut.clone();
                    new.player_at = step1;
                    if Some(new.player_at) == room_immut.key_at {
                        new.got_key = true;
                    }
                    try_add_room_mut(&mut search_nodes, &room_mut, direction, new);
                }
                if let Some(step2) = step1.take_step(direction) {
                    if room_mut.rocks_at.contains(step1)
                        && !room_mut.rocks_at.contains(step2)
                        && !room_immut.walls_at.contains(step2)
                        && Some(step2) != room_immut.lock_at
                    {
                        // player can kick rock from step1 to step2
                        let mut new = room_mut.clone();
                        new.rocks_at.remove(step1);
                        new.rocks_at.insert(step2);
                        try_add_room_mut(&mut search_nodes, &room_mut, direction, new);
                    }
                }
            }
        }
    }
    println!("took {:?}", start.elapsed());

    if let Some((_, n)) = best_solution.as_ref() {
        let mut pred_stack = vec![];
        let mut x: &RoomMut = n;
        while let Some((pred_room_mut, direction)) = &search_nodes.get(x).unwrap().pred {
            // continue building stack
            pred_stack.push((x, direction));
            x = pred_room_mut;
        }
        // start printing, unwinding stack
        // print root state (with no pred, reached by no directional move)
        printy(&room_immut, x);
        for (room_mut, direction) in pred_stack.iter().rev() {
            println!("input: \n{:?}", direction);
            printy(&room_immut, room_mut);
        }
    }
}
