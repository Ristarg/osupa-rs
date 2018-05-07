#![feature(proc_macro, wasm_custom_section, wasm_import_module)]

extern crate wasm_bindgen;
extern crate byteorder;
extern crate leb128;

use std::io::Cursor;
use byteorder::{LE, ReadBytesExt};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern {
    fn alert(s: &str);
}

#[wasm_bindgen]
pub fn do_stuff(buf: Vec<u8>) -> Vec<u32> {
    alert("In Rust!");
    alert(&format!("Got some bytes: {:?}", &buf[..20]));

    let db = OsuDb::parse(&buf);

    alert(&db.player_name);
    
    let ids = db.beatmaps.into_iter()
        .filter_map(|bm| match bm.beatmap_id {
            0 => None,
            _ => Some(bm.beatmap_id)
        })
        .collect::<Vec<u32>>();

    alert(&format!("From Rust: {:?}", &ids));
    ids
}

#[derive(Debug)]
struct OsuDb {
    version: u32,
    folder_count: u32,
    account_unlocked: bool,
    date_unlocked: u64, // DateTime type isn't explained in the spec
                        // it's 8 bytes so I assume it's a unix timestamp
    player_name: String,
    beatmaps: Vec<Beatmap>,
    unknown_int: u32
}

trait ReadOsuDbExt : ReadBytesExt {
    fn read_osu_byte(&mut self) -> u8 {
        self.read_u8().unwrap()
    }

    fn read_osu_boolean(&mut self) -> bool {
        self.read_osu_byte() != 0x00
    }

    fn read_osu_short(&mut self) -> u16 {
        self.read_u16::<LE>().unwrap()
    }

    fn read_osu_int(&mut self) -> u32 {
        self.read_u32::<LE>().unwrap()
    }

    fn read_osu_long(&mut self) -> u64 {
        self.read_u64::<LE>().unwrap()
    }

    fn read_osu_single(&mut self) -> f32 {
        self.read_f32::<LE>().unwrap()
    }

    fn read_osu_double(&mut self) -> f64 {
        self.read_f64::<LE>().unwrap()
    }

    fn read_osu_string(&mut self) -> String
        where Self: Sized {
        if self.read_osu_byte() == 0x00 {
            String::default()
        } else {
            let len = leb128::read::unsigned(self).unwrap();
            let mut buf = vec![0u8; len as usize];
            self.read_exact(&mut buf).unwrap();
            String::from_utf8(buf).unwrap()
        }
    }

    fn read_osu_int_double_pair(&mut self) -> IntDoublePair {
        IntDoublePair(
            {
                assert_eq!(self.read_osu_byte(), 0x08);
                self.read_osu_int()
            },
            {
                assert_eq!(self.read_osu_byte(), 0x0d);
                self.read_osu_double()
            }
        )
    }

    fn read_osu_timing_point(&mut self) -> TimingPoint {
        TimingPoint {
            bpm: self.read_osu_double(),
            offset: self.read_osu_double(),
            not_inherited: self.read_osu_boolean()
        }
    }
}

impl<R: ReadBytesExt + ?Sized> ReadOsuDbExt for R {}

impl OsuDb {
    fn parse(buf: &[u8]) -> Self {
        let mut rdr = Cursor::new(buf);
        OsuDb {
            version: rdr.read_osu_int(),
            folder_count: rdr.read_osu_int(),
            account_unlocked: rdr.read_osu_boolean(), //
            date_unlocked: rdr.read_osu_long(),       //TODO: combine into an Option-like enum?
            player_name: rdr.read_osu_string(),
            beatmaps: OsuDb::parse_beatmaps(&mut rdr),
            unknown_int: rdr.read_osu_int()
        }
    }

    fn parse_beatmaps(rdr: &mut Cursor<&[u8]>) -> Vec<Beatmap> {
        let number_of_beatmaps = rdr.read_osu_int();
        let mut beatmaps = Vec::with_capacity(number_of_beatmaps as usize);
        for _ in 0..number_of_beatmaps {
            beatmaps.push(Beatmap { // TODO: parallelize
                size_in_bytes: rdr.read_osu_int(),
                artist_name: rdr.read_osu_string(),
                artist_name_unicode: rdr.read_osu_string(),
                song_title: rdr.read_osu_string(),
                song_title_unicode: rdr.read_osu_string(),
                creator_name: rdr.read_osu_string(),
                difficulty: rdr.read_osu_string(),
                audio_file_name: rdr.read_osu_string(),
                md5_hash: rdr.read_osu_string(),
                osu_file_name: rdr.read_osu_string(),
                ranked_status: RankedStatus::from(rdr.read_osu_byte()),
                number_of_hitcircles: rdr.read_osu_short(),
                number_of_sliders: rdr.read_osu_short(),
                number_of_spinners: rdr.read_osu_short(),
                last_modified_winticks: rdr.read_osu_long(),
                approach_rate: rdr.read_osu_single(),
                circle_size: rdr.read_osu_single(),
                hp_drain: rdr.read_osu_single(),
                overall_difficulty: rdr.read_osu_single(),
                slider_velocity: rdr.read_osu_double(),
                star_rating_std: OsuDb::parse_int_double_pairs(rdr),
                star_rating_taiko: OsuDb::parse_int_double_pairs(rdr),
                star_rating_ctb: OsuDb::parse_int_double_pairs(rdr),
                star_rating_mania: OsuDb::parse_int_double_pairs(rdr),
                drain_time: rdr.read_osu_int(),
                total_time: rdr.read_osu_int(),
                audio_preview_offset: rdr.read_osu_int(),
                timing_points: {
                    let n = rdr.read_osu_int();
                    let mut points = Vec::with_capacity(n as usize);
                    for _ in 0..n {
                        points.push(rdr.read_osu_timing_point())
                    }
                    points
                }, //FIXME: this doesn't seem right, but the byte length matches alright, also code duplication
                beatmap_id: rdr.read_osu_int(),
                beatmap_set_id: rdr.read_osu_int(),
                thread_id: rdr.read_osu_int(),
                grade_achieved_std: rdr.read_osu_byte(),
                grade_achieved_taiko: rdr.read_osu_byte(),
                grade_achieved_ctb: rdr.read_osu_byte(),
                grade_achieved_mania: rdr.read_osu_byte(),
                local_offset: rdr.read_osu_short(),
                stack_leniency: rdr.read_osu_single(),
                gameplay_mode: GameplayMode::from(rdr.read_osu_byte()),
                song_source: rdr.read_osu_string(),
                song_tags: rdr.read_osu_string(),
                online_offset: rdr.read_osu_short(),
                title_font: rdr.read_osu_string(),
                is_unplayed: rdr.read_osu_boolean(),
                last_played: rdr.read_osu_long(),
                is_osz2: rdr.read_osu_boolean(),
                folder_name: rdr.read_osu_string(),
                last_checked_against_repo: rdr.read_osu_long(),
                ignore_sound: rdr.read_osu_boolean(),
                ignore_skin: rdr.read_osu_boolean(),
                disable_storyboard: rdr.read_osu_boolean(),
                disable_video: rdr.read_osu_boolean(),
                visual_override: rdr.read_osu_boolean(),
                last_modified: rdr.read_osu_int(),
                mania_scroll_speed: rdr.read_osu_byte()
            });
        }
        beatmaps
    }

    fn parse_int_double_pairs(rdr: &mut Cursor<&[u8]>) -> Vec<IntDoublePair> {
        let n = rdr.read_osu_int();
        let mut pairs = Vec::with_capacity(n as usize);
        for _ in 0..n {
            pairs.push(rdr.read_osu_int_double_pair());
        }
        pairs
    }
}

#[derive(Debug)]
struct Beatmap {
    size_in_bytes: u32,
    artist_name: String,
    artist_name_unicode: String,
    song_title: String,
    song_title_unicode: String,
    creator_name: String,
    difficulty: String,
    audio_file_name: String,
    md5_hash: String,
    osu_file_name: String,
    ranked_status: RankedStatus,
    number_of_hitcircles: u16,
    number_of_sliders: u16,
    number_of_spinners: u16,
    last_modified_winticks: u64,
    approach_rate: f32,
    circle_size: f32,
    hp_drain: f32,
    overall_difficulty: f32,
    slider_velocity: f64,
    star_rating_std: Vec<IntDoublePair>,
    star_rating_taiko: Vec<IntDoublePair>,
    star_rating_ctb: Vec<IntDoublePair>,
    star_rating_mania: Vec<IntDoublePair>,
    drain_time: u32,
    total_time: u32,
    audio_preview_offset: u32,
    timing_points: Vec<TimingPoint>,
    beatmap_id: u32,
    beatmap_set_id: u32,
    thread_id: u32,
    grade_achieved_std: u8,
    grade_achieved_taiko: u8,
    grade_achieved_ctb: u8,
    grade_achieved_mania: u8,
    local_offset: u16,
    stack_leniency: f32,
    gameplay_mode: GameplayMode,
    song_source: String,
    song_tags: String,
    online_offset: u16,
    title_font: String,
    is_unplayed: bool,
    last_played: u64,
    is_osz2: bool,
    folder_name: String,
    last_checked_against_repo: u64,
    ignore_sound: bool,
    ignore_skin: bool,
    disable_storyboard: bool,
    disable_video: bool,
    visual_override: bool,
    last_modified: u32,
    mania_scroll_speed: u8
}

#[derive(Debug)]
enum RankedStatus {
    Ranked = 4,
    Approved = 5,
    PendingOrGraveyard = 2,
    Undefined
}

impl From<u8> for RankedStatus {
    fn from(byte: u8) -> Self {
        match byte {
            2 => RankedStatus::PendingOrGraveyard,
            4 => RankedStatus::Ranked,
            5 => RankedStatus::Approved,
            _ => RankedStatus::Undefined
        }
    }
}

impl Default for RankedStatus {
    fn default() -> Self {
        RankedStatus::PendingOrGraveyard
    }
}

#[derive(Debug)]
enum GameplayMode {
    Standard = 0,
    Taiko = 1,
    CTB = 2,
    Mania = 3,
    Undefined
}

impl From<u8> for GameplayMode {
    fn from(byte: u8) -> Self {
        match byte {
            0 => GameplayMode::Standard,
            1 => GameplayMode::Taiko,
            2 => GameplayMode::CTB,
            3 => GameplayMode::Mania,
            _ => GameplayMode::Undefined
        }
    }
}

impl Default for GameplayMode {
    fn default() -> Self {
        GameplayMode::Standard
    }
}

#[derive(Debug)]
struct IntDoublePair(u32, f64);

#[derive(Debug)]
struct TimingPoint {
    bpm: f64,
    offset: f64,
    not_inherited: bool
}