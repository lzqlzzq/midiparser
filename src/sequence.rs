use std::collections::HashMap;
use crate::io::MIDIFile;
use crate::message::{MIDIMessage, MetaStatus, EventStatus};
use crate::util::tempo2qpm;

const DEFAULT_QPM: f32 = 120.0;
const DEFAULT_TEMPO: u32 = 500000;

#[derive(Clone, Debug)]
pub struct Sequence {
    pub tracks: Vec<Track>,
    pub time_signatures: Vec<TimeSignature>,
    pub key_signatures: Vec<KeySignature>,
    pub qpm: Vec<Tempo>,
}

#[derive(Clone, Debug, Default)]
pub struct Track {
    pub name: String,
    pub program: u8,
    pub is_drum: bool,
    pub notes: Vec<Note>,
    pub controls: HashMap<u8, Vec<ControlChange>>,
}

#[derive(Copy, Clone, Debug)]
pub struct Note {
    pub pitch: u8,
    pub start: f32,
    pub duration: f32,
    pub velocity: u8,
}

#[derive(Copy, Clone, Debug)]
pub struct ControlChange {
    pub time: f32,
    pub value: u8,
}

#[derive(Copy, Clone, Debug)]
pub struct TimeSignature {
    pub time: f32,
    pub numerator: u8,
    pub denominator: u8,
}

#[derive(Copy, Clone, Debug)]
pub struct KeySignature {
    pub time: f32,
    pub key: &'static str,
}

#[derive(Copy, Clone, Debug)]
pub struct Tempo {
    pub time: f32,
    pub qpm: f32,
}

impl Sequence {
    pub fn from_file(path: &str) -> Result<Sequence, &'static str> {
        let midi = MIDIFile::from_file(path)?;
        Self::from_midi(&midi)
    }
    pub fn from_midi(midi: &MIDIFile) -> Result<Sequence, &'static str> {
        if midi.division >> 15 == 1 {
            return Err("Division with 1 at high bit is not supported!");
        }
        let tpq = midi.division as f32; // ticks per quarter
        let mut qpm = Vec::new();
        let mut time_signatures = Vec::new();
        let mut key_signatures = Vec::new();
        let mut tracks = HashMap::<(u8, u8), Track>::new();
        let mut track_names = vec![String::new(); midi.tracks.len()];
        for (track_idx, track) in midi.tracks.iter().enumerate() {
            let mut cur_instr = [0_u8; 16]; // 16 channels
            let mut last_note_on = [[(0_u32, 0_u8); 128]; 16]; // （start, velocity)
            for msg in track.iter() {
                match msg {
                    MIDIMessage::Event(event) => {
                        let cur = event.time as f32 / tpq;
                        match event.status {
                            EventStatus::ProgramChange => {
                                cur_instr[event.channel().unwrap_or(0) as usize]
                                    = event.program().unwrap_or(0)
                            }
                            EventStatus::ControlChange => {
                                let channel = event.channel().unwrap_or(0);
                                let track_entry = tracks
                                    .entry((track_idx as u8, channel))
                                    .or_insert(Track {
                                        program: cur_instr[channel as usize],
                                        is_drum: channel == 9,
                                        ..Track::default()
                                    });
                                let (ctrl_k, ctrl_v) = event.control_change().unwrap();
                                let ctrl_entry = track_entry
                                    .controls.entry(ctrl_k)
                                    .or_insert(Vec::new());
                                ctrl_entry.push(ControlChange {
                                    time: cur,
                                    value: ctrl_v,
                                });
                            }
                            EventStatus::NoteOn | EventStatus::NoteOff => {
                                let velocity = event.velocity().unwrap_or(0);
                                let channel = event.channel().unwrap_or(0);
                                let pitch = event.key().unwrap();
                                // NoteOff
                                if velocity == 0 || event.status == EventStatus::NoteOff {
                                    let (start, on_vel) = last_note_on[channel as usize][pitch as usize];
                                    if on_vel != 0 {
                                        let track_entry = tracks
                                            .entry((track_idx as u8, channel))
                                            .or_insert(Track {
                                                program: cur_instr[channel as usize],
                                                is_drum: channel == 9,
                                                ..Track::default()
                                            });
                                        track_entry.notes.push(Note {
                                            pitch,
                                            velocity: on_vel,
                                            start: start as f32 / tpq,
                                            duration: (event.time - start) as f32 / tpq,
                                        });
                                        last_note_on[channel as usize][pitch as usize].1 = 0;
                                    }
                                } else {
                                    last_note_on[channel as usize][pitch as usize] = (event.time, velocity);
                                }
                            }
                            _ => {} // Pass unused event
                        }
                    }
                    MIDIMessage::Meta(meta) => {
                        let cur = meta.time as f32 / tpq;
                        match meta.status {
                            MetaStatus::SetTempo => {
                                qpm.push(Tempo {
                                    time: cur,
                                    qpm: tempo2qpm(meta.tempo().unwrap_or(DEFAULT_TEMPO)),
                                })
                            }
                            MetaStatus::TimeSignature => {
                                let t = meta.time_signature().unwrap_or((4, 4, 0, 0));
                                time_signatures.push(TimeSignature {
                                    time: cur,
                                    numerator: t.0,
                                    denominator: t.1,
                                })
                            }
                            MetaStatus::KeySignature => {
                                key_signatures.push(KeySignature {
                                    time: cur,
                                    key: meta.key_signature().unwrap(),
                                })
                            }
                            MetaStatus::TrackName => {
                                let name: String = String::from_utf8(
                                    meta.meta_value().to_vec()
                                ).unwrap();
                                track_names[track_idx] = name;
                            }
                            _ => {} // Pass unknown meta
                        }
                    }
                }
            }
        }

        qpm.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
        time_signatures.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
        key_signatures.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
        if qpm.is_empty() || qpm[0].time > 0.0 {
            qpm.insert(0, Tempo { time: 0.0, qpm: DEFAULT_QPM });
        }
        println!("Track Num: {}", tracks.len());
        Ok(Sequence {
            tracks: tracks
                .into_iter()
                .map(|(k, mut t)| {
                    t.name = track_names[k.0 as usize].clone();
                    t
                }) // .filter(|t| !t.notes.is_empty())
                .collect(),
            time_signatures,
            key_signatures,
            qpm,
        })
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_midi2seq() {
        let seq = Sequence::from_file("tests/tiny.mid").unwrap();
        println!("Time Signature: {:?}", seq.time_signatures);
        println!("Key Signature: {:?}", seq.key_signatures);
        println!("QPM {:?}", seq.qpm);
        for track in seq.tracks {
            println!("Track: {}", track.name);
            println!("{:?}", track);
        }
    }
}