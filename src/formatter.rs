use fraction::Fraction
use crate::mesage::{ EventStatus, MIDIMessage, MIDIFormat, MetaStatus };
use crate::io:: { MIDITrack, MIDIFile }

pub struct Note {
	pitch: u8,
	start: Fraction,
	duration: Fraction,
}

pub struct ControlChange {
	time: Fraction,
	control: u8,
	value: u8,
}

#[derive(Clone, Debug)]
pub struct Track {
	pub name: String,
	pub program: u8,
	pub notes: Vec<Note>,
}

pub struct Sequence {
	pub tracks: Vec<Track>
}
