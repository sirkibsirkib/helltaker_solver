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

struct RoomMutEdge {
    predecessor: RoomMut,
    step_direction: Direction,
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
        if !c.is_within_bounds() {
            return None;
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
impl Direction {
    fn all_directions() -> impl Iterator<Item = Self> {
        [Direction::Up, Direction::Down, Direction::Left, Direction::Right].iter().copied()
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

fn print_solution_path(
    i: &RoomImmut,
    end: &RoomMut,
    room_graph: &fxhash::FxHashMap<RoomMut, Option<RoomMutEdge>>,
) {
    let mut node = end;
    let mut pred_stack = vec![];
    while let Some(room_mut_edge) = room_graph.get(node).unwrap().as_ref() {
        // continue building stack
        pred_stack.push((node, room_mut_edge.step_direction));
        node = &room_mut_edge.predecessor;
    }
    // start printing, unwinding stack
    // print root state (with no pred, reached by no directional move)
    printy(i, node);
    for ((room_mut, direction), step_num) in pred_stack.into_iter().rev().zip(1..) {
        println!("\nstep: {}, input: {:?}", step_num, direction);
        printy(i, room_mut);
    }
}

fn resulting_room_mut(i: &RoomImmut, m: &RoomMut, direction: Direction) -> Option<RoomMut> {
    if let Some(step1) = m.player_at.take_step(direction) {
        if !i.walls_at.contains(step1)
            && !m.rocks_at.contains(step1)
            && (m.got_key || Some(step1) != i.lock_at)
        {
            // player can move to `step1`
            let mut new = m.clone();
            new.player_at = step1;
            if Some(new.player_at) == i.key_at {
                new.got_key = true;
            }
            return Some(new);
        }
        if let Some(step2) = step1.take_step(direction) {
            if m.rocks_at.contains(step1)
                && !m.rocks_at.contains(step2)
                && !i.walls_at.contains(step2)
                && Some(step2) != i.lock_at
            {
                // player can kick rock from step1 to step2
                let mut new = m.clone();
                new.rocks_at.remove(step1);
                new.rocks_at.insert(step2);
                return Some(new);
            }
        }
    }
    None
}

fn main() {
    let (room_immut, init_room_mut) = parse_init(INIT_ROOM_BYTE_STR).unwrap();

    let mut room_graph = fxhash::FxHashMap::<RoomMut, Option<RoomMutEdge>>::default();
    room_graph.insert(init_room_mut.clone(), None);

    let start = std::time::Instant::now();

    // Invariant: no duplication of contents in (to_visit U visiting)
    //   elements only added with addition to new keys in `room_graph`.
    let mut visiting = Vec::with_capacity(128);
    let mut to_visit = Vec::with_capacity(128);
    visiting.push(init_room_mut);

    for steps_taken in 0.. {
        // println!("steps taken {}. visiting {}", steps_taken, visiting.len());
        if visiting.is_empty() {
            println!("No solutions");
            return;
        }
        for room_mut in visiting.drain(..) {
            for direction in Direction::all_directions() {
                if let Some(new_room_mut) = resulting_room_mut(&room_immut, &room_mut, direction) {
                    use std::collections::hash_map::Entry;
                    if let Entry::Vacant(ve) = room_graph.entry(new_room_mut.clone()) {
                        ve.insert(Some(RoomMutEdge {
                            predecessor: room_mut.clone(),
                            step_direction: direction,
                        }));
                        if new_room_mut.player_at == room_immut.goal_at {
                            // found a solution!
                            println!("Took {:?}", start.elapsed());
                            print_solution_path(&room_immut, &room_mut, &room_graph);
                            return;
                        } else {
                            to_visit.push(new_room_mut);
                        }
                    }
                }
            }
        }
        std::mem::swap(&mut to_visit, &mut visiting);
    }
}
