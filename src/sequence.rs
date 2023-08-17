use crate::message::{ EventStatus, MetaStatus };
use crate::io::MIDIFile;
use std::collections::HashMap;

const DEFAULT_QPM :f32 = 120.0;
const DEFAULT_TEMPO: u32 = 500000;


#[derive(Clone, Debug)]
pub struct Note {
	pub pitch: u8,
	pub start: f32,
	pub duration: f32,
	pub velocity: u8,
}

#[derive(Clone, Debug)]
pub struct ControlChange {
	pub time: f32,
	pub value: u8,
}

#[derive(Clone, Debug)]
pub struct TimeSignature{
	pub time: f32,
	pub numerator: u8,
	pub denominator: u8,
}

#[derive(Clone, Debug)]
pub struct KeySignature{
	pub time: f32,
	pub key: i8,
}

#[derive(Clone, Debug)]
pub struct Tempo{
	pub time: f32,
	pub qpm: f32,
}

#[derive(Clone, Debug)]
pub struct Track {
	pub name: String,
	pub program: u8,
	pub is_drum: bool,
	pub notes: Vec<Note>,
	pub controls: HashMap<u8, Vec<ControlChange>>,
}

#[derive(Clone, Debug)]
pub struct Sequence {
	pub tracks: Vec<Track>,
	pub time_signatures: Vec<TimeSignature>,
	pub key_signatures: Vec<KeySignature>,
	pub qpm: Vec<Tempo>,
}


#[inline(always)]
pub fn tempo2qpm(tempo: u32) -> f32 {
	6e7 / tempo as f32
}

impl Sequence{
	pub fn from_midi(midi: &MIDIFile) -> Sequence{
		if (midi.division >> 15) == 1 {panic!()}
		let tpq = midi.division as f32;
		let mut qpm = Vec::new();
		let mut time_signatures = Vec::new();
		let mut key_signatures = Vec::new();
		let mut tracks = HashMap::<(u8, u8), Track>::new(); // (track_idx, channel) -> Track
		let mut track_names = vec![String::new(); midi.track.len()];

		for (track_idx, track) in midi.track.iter().enumerate() {
			let track_idx = track_idx as u8;
			let mut cur_instr = [0_u8; 16];
			let mut last_note_on = [[(0_u32, 0_u8); 128]; 16];
			for msg in track.message.iter(){
				match msg.status {
					EventStatus::Meta => {
						match msg.meta_type().unwrap() {
							MetaStatus::SetTempo => {
								qpm.push(Tempo{
									time: msg.time as f32 / tpq,
									qpm: tempo2qpm(msg.tempo_change().unwrap_or(DEFAULT_TEMPO))
								});
							},
							MetaStatus::TimeSignature => {
								let t = msg.time_signature().unwrap_or((4, 4, 0, 0));
								time_signatures.push(TimeSignature{
									time: msg.time as f32 / tpq,
									numerator: t.0,
									denominator: t.1
								});
							},
							MetaStatus::KeySignature => {
									key_signatures.push(KeySignature{
									time: msg.time as f32 / tpq,
									key: 0 // TODO
								});
							},
							MetaStatus::TrackName => {
								let name: String = String::from_utf8(
									msg.meta_value()
									.unwrap_or(Vec::new())
								).unwrap_or(String::new());
								track_names[track_idx as usize] = name;
							},
							_ => {}
						}
					},
					EventStatus::ProgramChange => {
						cur_instr[msg.channel().unwrap_or(0) as usize] 
							= msg.program().unwrap_or(0);
					},
					EventStatus::ControlChange => {
						let channel = msg.channel().unwrap_or(0);
						let track_entry = tracks
							.entry((track_idx, channel))
							.or_insert(Track{
								name: String::new(),
								program: cur_instr[channel as usize],
								is_drum: channel == 9,
								notes: Vec::new(),
								controls: HashMap::new()
							});
						let entry = track_entry
							.controls.entry(msg.controller_change().unwrap())
							.or_insert(Vec::new());
						entry.push(ControlChange{
							time: msg.time as f32 / tpq,
							value: msg.controller_change_value().unwrap()
						})
					}
					EventStatus::NoteOn | EventStatus::NoteOff => {
						let velocity = msg.velocity().unwrap_or(0);
						let channel = msg.channel().unwrap_or(0);
						let pitch = msg.key().unwrap();
						if velocity == 0 || msg.status == EventStatus::NoteOff {
							// note off
							let (start, vel) = last_note_on[channel as usize][pitch as usize];
							if vel != 0 {
								let entry = tracks.entry((track_idx, channel))
									.or_insert(Track{
										name: String::new(),
										program: cur_instr[channel as usize],
										is_drum: channel == 9,
										notes: Vec::new(),
										controls: HashMap::new()
									});
								entry.notes.push(Note{
									pitch,
									start: start as f32 / tpq,
									duration: (msg.time - start) as f32 / tpq,
									velocity: vel
								});
								last_note_on[channel as usize][pitch as usize].1 = 0;
							} 
						} else {	// Note on
							last_note_on[channel as usize][pitch as usize] = (
								msg.time, velocity
							);
						}
					},
					_ => {}
				}
			}
		}
		qpm.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
		time_signatures.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
		key_signatures.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
		if qpm.is_empty() || qpm[0].time > 0.0 {
			qpm.insert(0, Tempo {time:0.0, qpm: DEFAULT_QPM});
		}
		Sequence{
			tracks: tracks
				.into_iter()
				.map(|(k, mut t)| {
					t.name = track_names[k.0 as usize].clone(); t
				}).filter(|t| !t.notes.is_empty())
				.collect(),
			time_signatures,
			key_signatures,
			qpm
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_midi2seq() {
		let mf = MIDIFile::read_midi_file("tests/tiny.mid").expect("Read midi failed.");
		let seq = Sequence::from_midi(&mf);
		println!("Time Signature: {:?}", seq.time_signatures);
		println!("Key Signature: {:?}", seq.key_signatures);
		println!("QPM {:?}", seq.qpm);
		for track in  seq.tracks {
			println!("Track: {}", track.name);
			println!("{:?}", track);
		}
	}
}