use std::collections::HashMap;
use std::{collections::BTreeMap, rc::Rc};

fn reverse_dir(dir: &str) -> String {
    match dir {
        "n" => "s",
        "s" => "n",
        "e" => "w",
        "w" => "e",
        "ne" => "sw",
        "sw" => "ne",
        "se" => "nw",
        "nw" => "se",
        "u" => "d",
        "d" => "u",
        &_ => dir,
    }
    .to_string()
}

#[derive(Default, Debug, PartialEq, Eq, Hash, Clone, Copy)]
struct Coordinate {
    x: i64,
    y: i64,
    z: i64,
}

impl Coordinate {
    fn _new(x: i64, y: i64, z: i64) -> Self {
        Self { x, y, z }
    }

    fn get_next(&self, dir: &str) -> Self {
        let mut new_cord = *self;
        match dir {
            "n" => new_cord.y += 1,
            "e" => new_cord.x += 1,
            "s" => new_cord.y -= 1,
            "w" => new_cord.x -= 1,
            "u" => new_cord.z += 1,
            "d" => new_cord.z -= 1,
            "ne" => {
                new_cord.y += 1;
                new_cord.x += 1;
            }
            "se" => {
                new_cord.y -= 1;
                new_cord.x += 1;
            }
            "sw" => {
                new_cord.y -= 1;
                new_cord.x -= 1;
            }
            "nw" => {
                new_cord.y += 1;
                new_cord.x -= 1;
            }
            &_ => {}
        }
        new_cord
    }
}

#[derive(Debug)]
pub struct Map {
    rooms: Vec<Rc<Room>>,
    grid: HashMap<Coordinate, Rc<Room>>,
    current_room: Option<usize>,
}

#[allow(dead_code)]
impl Map {
    fn new() -> Self {
        let room = Rc::new(Room::create(0, Coordinate::default()));
        let rooms = vec![Rc::clone(&room)];
        let mut grid = HashMap::new();
        grid.insert(room.coord, room);

        Self {
            rooms,
            grid,
            current_room: Some(0),
        }
    }

    fn travel(&mut self, dir: &str) {
        if let Some(current) = self.current_room {
            let current = &self.rooms[current];
            if let Some(id) = current.exits.get(dir) {
                self.current_room.replace(*id);
            } else {
                self.current_room = self.add_room(dir);
            }
        }
    }

    fn add_room(&mut self, dir: &str) -> Option<usize> {
        if let Some(current) = self.current_room {
            let coord = self.rooms[current].coord.get_next(dir);
            let room_id = if let Some(room) = self.grid.get(&coord) {
                room.id
            } else {
                let id = self.rooms.len();
                let room = Rc::new(Room::create(id, coord));
                self.rooms.push(room.clone());
                self.grid.insert(room.coord, room);
                id
            };
            Rc::make_mut(&mut self.rooms[room_id]).add_exit(&reverse_dir(dir), current);
            Rc::make_mut(&mut self.rooms[current]).add_exit(dir, room_id);
            Some(room_id)
        } else {
            None
        }
    }
}

#[derive(Debug, Default, Clone)]
struct Room {
    id: usize,
    coord: Coordinate,
    name: String,
    exits: BTreeMap<String, usize>,
}

#[allow(dead_code)]
impl Room {
    fn create(id: usize, coord: Coordinate) -> Self {
        Self {
            id,
            coord,
            name: String::new(),
            exits: BTreeMap::new(),
        }
    }

    fn add_exit(&mut self, dir: &str, id: usize) {
        if !self.exits.contains_key(dir) {
            self.exits.insert(dir.to_string(), id);
        }
    }
}

#[cfg(test)]
mod map_tests {

    use super::*;

    fn check_exit(room: &Room, dir: &str, id: usize) -> bool {
        if let Some(room_id) = room.exits.get(dir) {
            *room_id == id
        } else {
            false
        }
    }

    #[test]
    fn test_next_coord() {
        let coord = Coordinate::default();
        assert_eq!(Coordinate::_new(0, 1, 0), coord.get_next("n"));
        assert_eq!(Coordinate::_new(1, 0, 0), coord.get_next("e"));
        assert_eq!(Coordinate::_new(0, -1, 0), coord.get_next("s"));
        assert_eq!(Coordinate::_new(-1, 0, 0), coord.get_next("w"));
        assert_eq!(Coordinate::_new(1, 1, 0), coord.get_next("ne"));
        assert_eq!(Coordinate::_new(1, -1, 0), coord.get_next("se"));
        assert_eq!(Coordinate::_new(-1, -1, 0), coord.get_next("sw"));
        assert_eq!(Coordinate::_new(-1, 1, 0), coord.get_next("nw"));
        assert_eq!(Coordinate::_new(0, 0, 1), coord.get_next("u"));
        assert_eq!(Coordinate::_new(0, 0, -1), coord.get_next("d"));
    }

    #[test]
    fn test_reverse_dir() {
        assert_eq!(reverse_dir("n"), "s");
        assert_eq!(reverse_dir("s"), "n");
        assert_eq!(reverse_dir("e"), "w");
        assert_eq!(reverse_dir("w"), "e");
        assert_eq!(reverse_dir("nw"), "se");
        assert_eq!(reverse_dir("ne"), "sw");
        assert_eq!(reverse_dir("se"), "nw");
        assert_eq!(reverse_dir("sw"), "ne");
        assert_eq!(reverse_dir("u"), "d");
        assert_eq!(reverse_dir("d"), "u");
    }

    #[test]
    fn test_cycle_detection() {
        let mut map = Map::new();
        assert_eq!(map.rooms.len(), 1);
        map.travel("s");
        assert_eq!(map.rooms.len(), 2);
        map.travel("ne");
        assert_eq!(map.rooms.len(), 3);
        map.travel("w");
        assert_eq!(map.rooms.len(), 3);

        assert!(check_exit(&map.rooms[0], "s", 1));
        assert!(check_exit(&map.rooms[1], "ne", 2));
        assert!(check_exit(&map.rooms[2], "w", 0));
    }

    #[test]
    fn test_reverse_direction() {
        let mut map = Map::new();
        assert_eq!(map.rooms.len(), 1);
        map.travel("s");
        assert_eq!(map.rooms.len(), 2);
        map.travel("ne");
        assert_eq!(map.rooms.len(), 3);
        map.travel("w");
        assert_eq!(map.rooms.len(), 3);

        assert!(check_exit(&map.rooms[0], "e", 2));
        assert!(check_exit(&map.rooms[2], "sw", 1));
        assert!(check_exit(&map.rooms[1], "n", 0));
    }
}
