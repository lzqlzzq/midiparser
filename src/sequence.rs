use std::collections::HashMap;
use std::fmt::Debug;
use pyo3::exceptions::{PyIOError};
use pyo3::prelude::*;
use crate::io::MIDIFile;
use crate::message::{MIDIMessage, MetaStatus, EventStatus};
use crate::util::tempo2qpm;
use serde::{Serialize, Deserialize};
use serde_yaml;

const DEFAULT_QPM: f32 = 120.0;
const DEFAULT_TEMPO: u32 = 500000;

#[pyclass]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Sequence {
    #[pyo3(get, set)]
    pub tracks: Vec<Track>,
    #[pyo3(get, set)]
    pub time_signatures: Vec<TimeSignature>,
    #[pyo3(get, set)]
    pub key_signatures: Vec<KeySignature>,
    #[pyo3(get, set)]
    pub qpm: Vec<Tempo>,
}
#[pyclass]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Track {
    #[pyo3(get, set)]
    pub name: String,
    #[pyo3(get, set)]
    pub program: u8,
    #[pyo3(get, set)]
    pub is_drum: bool,
    #[pyo3(get, set)]
    pub notes: Vec<Note>,
    #[pyo3(get, set)]
    pub controls: HashMap<u8, Vec<ControlChange>>,
}
#[pyclass]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TrackTrans {
    #[pyo3(get, set)]
    pub name: String,
    #[pyo3(get, set)]
    pub program: u8,
    #[pyo3(get, set)]
    pub is_drum: bool,
    #[pyo3(get, set)]
    pub pitch: Vec<u8>,
    #[pyo3(get, set)]
    pub start: Vec<f32>,
    #[pyo3(get, set)]
    pub duration: Vec<f32>,
    #[pyo3(get, set)]
    pub velocity: Vec<u8>,
    #[pyo3(get, set)]
    pub controls: HashMap<u8, Vec<ControlChange>>,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
#[pyclass]
pub struct Note {
    #[pyo3(get, set)]
    pub pitch: u8,
    #[pyo3(get, set)]
    pub start: f32,
    #[pyo3(get, set)]
    pub duration: f32,
    #[pyo3(get, set)]
    pub velocity: u8,
}

#[pyclass]
#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct ControlChange {
    #[pyo3(get, set)]
    pub time: f32,
    #[pyo3(get, set)]
    pub value: u8,
}

#[pyclass]
#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct TimeSignature {
    #[pyo3(get, set)]
    pub time: f32,
    #[pyo3(get, set)]
    pub numerator: u8,
    #[pyo3(get, set)]
    pub denominator: u8,
}

#[pyclass]
#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct KeySignature {
    #[pyo3(get, set)]
    pub time: f32,
    #[pyo3(get, set)]
    pub key: (bool, i8), // bool true 代表大调，false代表小调
}

#[pyclass]
#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct Tempo {
    #[pyo3(get, set)]
    pub time: f32,
    #[pyo3(get, set)]
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

#[pymethods]
impl Sequence {
    #[new]
    pub fn py_new(path: &str) -> PyResult<Self> {
        let seq = Self::from_file(path);
        match seq {
            Err(info) => Err(PyIOError::new_err(info)),
            Ok(seq) => Ok(seq)
        }
    }
    pub fn __repr__(&self) -> String {
        serde_yaml::to_string(&self).unwrap()
    }
}

impl Track {
    pub fn transpose(&self) -> TrackTrans {
        let mut pitch = Vec::with_capacity(self.notes.len());
        let mut start = Vec::with_capacity(self.notes.len());
        let mut duration = Vec::with_capacity(self.notes.len());
        let mut velocity = Vec::with_capacity(self.notes.len());
        for note in &self.notes {
            pitch.push(note.pitch);
            start.push(note.start);
            duration.push(note.duration);
            velocity.push(note.velocity);
        } TrackTrans{
            pitch, start, duration, velocity,
            program: self.program,
            is_drum: self.is_drum,
            name: self.name.clone(),
            controls: self.controls.clone()
        }
    }
}

#[pymethods]
impl Track {
    pub fn __repr__(&self) -> String {
        serde_yaml::to_string(&self).unwrap()
    }

    #[pyo3(name="transpose")]
    pub fn py_transpose(&self) -> TrackTrans {self.transpose()}
}

#[pymethods]
impl TrackTrans {
    fn __repr__(&self) -> String { return format!("{:?}", self) }
}
#[pymethods]
impl Note {
    fn __repr__(&self) -> String { return format!("{:?}", self) }
}

#[pymethods]
impl TimeSignature {
    fn __repr__(&self) -> String { return format!("{:?}", self) }
}

#[pymethods]
impl KeySignature {
    fn __repr__(&self) -> String { return format!("{:?}", self) }
}

#[pymethods]
impl ControlChange {
    fn __repr__(&self) -> String { return format!("{:?}", self) }
}

#[pymethods]
impl Tempo {
    fn __repr__(&self) -> String { return format!("{:?}", self) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;
    use serde_yaml;
    #[test]
    fn test_midi2seq() {
        let seq = Sequence::from_file("tests/tiny.mid").unwrap();
        let t= serde_yaml::to_string(&seq).unwrap();
        println!("{t}");
    }
}