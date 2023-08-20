use std::fs;
use std::str;
use crate::message::{MIDIFormat, EventStatus, MIDIMessage, MetaStatus};
use crate::util::read_variable_length;

#[derive(Clone)]
pub struct MIDIFile {
    pub format: MIDIFormat,
    pub division: u16,
    pub tracks: Vec<MidiTrack>,
}

#[derive(Clone)]
pub struct MidiTrack {
    track_idx: u16,
    data: Vec<u8>,
}

pub struct MidiTrackIter<'a> {
    data: &'a [u8],

    byte_offset: usize,
    tick_offset: u32,

    last_status_code: u8,
    last_event_len: usize,
}

impl MidiTrack {
    pub fn iter(&self) -> MidiTrackIter {
        MidiTrackIter {
            data: &self.data,
            byte_offset: 0,
            tick_offset: 0,
            last_event_len: 0,
            last_status_code: 0,
        }
    }
}

impl MIDIFile {
    pub fn from_file(path: &str) -> Result<MIDIFile, &'static str> {
        let data = fs::read(path)
            .expect(concat!("Can not read file ", stringify!(path)));
        assert!(&data.starts_with(b"MThd"), "Invalid midi file. MThd expected.");
        let (format, track_num, division) = Self::parse_mthd(&data[8..14]);
        let mut midi = MIDIFile {
            format,
            division,
            tracks: Vec::new(),
        };
        let mut byte_offset = 14;

        for track_idx in 0..track_num {
            let mut chunk_len = u32::from_be_bytes(
                data[byte_offset + 4..byte_offset + 8]
                    .try_into().expect("Invalid chunk!")
            );
            // Skip unknown chunks
            while !data[byte_offset..].starts_with(b"MTrk") {
                byte_offset += 8 + chunk_len as usize;
                chunk_len = u32::from_be_bytes(
                    data[byte_offset + 4..byte_offset + 8]
                        .try_into().expect("Invalid chunk!")
                )
            }
            let start = byte_offset + 8;
            let end = start + chunk_len as usize;
            byte_offset = end;
            midi.tracks.push(MidiTrack {
                track_idx,
                data: data[start..end].to_vec(),
            });
        }
        Ok(midi)
    }

    fn parse_mthd(data: &[u8]) -> (MIDIFormat, u16, u16) {
        let to_u16 = |s: &[u8]|
            u16::from_be_bytes(s.try_into().expect("Error reading midi file."));
        let format = match to_u16(&data[0..2]) {
            0 => MIDIFormat::SingleTrack,
            1 => MIDIFormat::MultiTrack,
            2 => MIDIFormat::MultiSong,
            x => panic!("MIDI format {} is not supported.", x),
        };
        (format, to_u16(&data[2..4]), to_u16(&data[4..6]))
    }
}

impl<'a> Iterator for MidiTrackIter<'a> {
    type Item = MIDIMessage;

    fn next(&mut self) -> Option<Self::Item> {
        if self.byte_offset >= self.data.len() { return None; }
        let (bytes, value) = read_variable_length(
            &self.data[self.byte_offset..self.byte_offset + 4]
                .try_into()
                .expect("Reading variable length error.")
        );
        self.byte_offset += bytes as usize;
        self.tick_offset += value as u32;

        let this_status: u8 = self.data[self.byte_offset];
        let start = self.byte_offset;
        let msg = match this_status {
            // MIDI Messages and SysEx Messages has determinate length.
            0xF0 | 0xF7 => {
                let (bytes, mut event_len) = read_variable_length(
                    match self.data.get(start + 1..start + 5) {
                        Some(res) => res.try_into().unwrap(),
                        None => &[0u8; 4]
                    }
                );
                event_len += bytes as usize + 1;
                self.byte_offset += event_len;
                self.last_event_len = event_len;
                self.last_status_code = this_status;
                let end_byte = self.data[self.byte_offset - 1];
                assert_eq!(end_byte, 0xf7_u8);
                return self.next();
            }
            0x00..=0x7F => {
                assert_ne!(self.last_status_code, 0xFF, "Last status can't be meta");
                self.byte_offset += self.last_event_len - 1;
                MIDIMessage::new_event(
                    self.tick_offset,
                    self.last_status_code,
                    &self.data[start..self.byte_offset],
                ) // data 部分统一不包含 status code，在 new 函数中统一拼接
            }
            0x80..=0xFE => {
                self.last_status_code = this_status;
                let event_len = EventStatus::from_status_code(this_status).1 as usize;
                self.byte_offset += event_len;
                self.last_event_len = event_len;
                MIDIMessage::new_event(
                    self.tick_offset,
                    this_status,
                    &self.data[start + 1..self.byte_offset],
                )
            }
            // Meta Messages has variable length.
            0xFF => {
                let (bytes, mut meta_len) = read_variable_length(
                    match self.data.get(start + 2..start + 6) {
                        Some(res) => res.try_into().unwrap(),
                        None => &[0u8; 4]
                    }
                );
                meta_len += bytes as usize + 2;
                self.byte_offset += meta_len;
                MIDIMessage::new_meta(
                    self.tick_offset,
                    this_status,
                    &self.data[start + 1..self.byte_offset],
                )
            }
        };
        Some(msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_midi_head() {
        let mf = MIDIFile::from_file("tests/tiny.mid").expect("Read midi failed.");
        assert!(mf.format == MIDIFormat::MultiTrack);
        println!("{:?}", mf.tracks.len());
        println!("{:?}", mf.division);
        for t in mf.tracks {
            for m in t.iter() {
                match m {
                    MIDIMessage::Event(event) => {
                        println!("{:?}: {:?}", event.time, event.data);
                    }
                    MIDIMessage::Meta(meta) => {
                        if meta.status == MetaStatus::SetTempo {
                            println!("tempo {:?}", meta.tempo().unwrap());
                        }
                    }
                }
            }
        }
    }
}