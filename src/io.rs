use std::fs;
use std::str;
use crate::message::{ EventStatus, MIDIMessage, MIDIFormat, MetaStatus};
use crate::util:: read_variable_length;

# [derive(Clone)]
pub struct MIDIMessageIter<'a> {
    data: &'a [u8],
    bytes: usize,

    byte_offset: usize,
    tick_offset: u32,

    last_status: EventStatus,
    last_event_len: usize,
    last_status_code: u8,
}

impl MIDIMessageIter<'_> {
    pub fn from_bytes(data: &[u8], bytes: usize) -> MIDIMessageIter {
        MIDIMessageIter {
            data: data,
            bytes: bytes,

            byte_offset: 0,
            tick_offset: 0,

            last_status: EventStatus::Meta,
            last_event_len: 0,
            last_status_code: 0,
        }
    }
}

impl Iterator for MIDIMessageIter<'_> {
    type Item = MIDIMessage;

    fn next(&mut self) -> Option<MIDIMessage> {
        if(self.byte_offset >= self.bytes) { return None };

        let (bytes, value) = read_variable_length(&self.data[self.byte_offset..self.byte_offset+4].try_into().expect("Reading variable length error."));
        self.byte_offset += bytes as usize;
        self.tick_offset += value as u32;
        let this_status = &self.data[self.byte_offset];

        let (event_status, message_data, length_to_offset) = match &this_status {
            // Running status of MIDI Messages has original length - 1.
            0x00..=0x7F => {
                let mut message = vec![self.last_status_code];
                message.extend_from_slice(&self.data[self.byte_offset..self.byte_offset+self.last_event_len-1]);
                (self.last_status, message, self.last_event_len - 1)
            },
            // MIDI Messages and SysEx Messages has determinated length.
            0x80..=0xFE => {
                self.last_status_code = *this_status;
                let (status, event_len) = EventStatus::from_status_code(&self.data[self.byte_offset]);
                (self.last_status, self.last_event_len) = (status, event_len as usize);
                (status, Vec::from(&self.data[self.byte_offset..self.byte_offset+(event_len as usize)]), event_len as usize)
            },
            // Meta Messages has variable length.
            0xFF => {
                let (meta_length_bytes, mut metalen) = read_variable_length(match self.data.get(self.byte_offset+2..self.byte_offset+6)
                {
                    Some(result) => result.try_into().unwrap(),
                    None => &[0u8, 0u8, 0u8, 0u8],
                });
                metalen += (meta_length_bytes as usize) + 2;
                (EventStatus::from_status_code(this_status).0, Vec::from(&self.data[self.byte_offset..self.byte_offset+metalen]), metalen)
            },
        };

        self.byte_offset += length_to_offset;

        Some(MIDIMessage {
            time: self.tick_offset,
            status: event_status,
            data: message_data,
        })
    }
}

pub struct MIDITrackIter {
    data: Vec<u8>,
    byte_offset: usize,
    track_num: u16,
    cur_track_idx: u16,
}

impl MIDITrackIter {
    pub fn from_bytes(data: Vec<u8>, track_num: u16) -> MIDITrackIter {
        MIDITrackIter {
            data: data,
            byte_offset: 14,
            track_num: track_num,
            cur_track_idx: 0,
        }
    }
}

impl Iterator for MIDITrackIter {
    type Item = MIDIMessageIter;

    fn next(&mut self) -> Option<MIDIMessageIter> {
        if(self.cur_track_idx == self.track_num) { return None };

        let chunk_length = u32::from_be_bytes(self.data[self.byte_offset+4..self.byte_offset+8].try_into().expect("Invaild chunk!")) as usize;

        // Skip unknown chunks
        while !(self.data[self.byte_offset..]).starts_with(b"MTrk") {
            self.byte_offset += 8 + chunk_length;
            chunk_length = u32::from_be_bytes(self.data[self.byte_offset+4..self.byte_offset+8].try_into().expect("Invaild chunk!")) as usize;
        }

        let message_iter = MIDIMessageIter::from_bytes(&self.data[self.byte_offset+8..self.byte_offset+8+chunk_length], chunk_length);
        
        // Move track pointer and byte pointer.
        self.cur_track_idx += 1;
        self.byte_offset += 8 + chunk_length;

        Some(message_iter)
    }
}

#[derive(Clone)]
pub struct MIDITrack {
    pub message: Vec<MIDIMessage>,
}

#[derive(Clone)]
pub struct MIDIFile {
    pub format: MIDIFormat,
    pub track_num: u16,
    pub division: u16,
    pub track: Vec<MIDITrack>,
}

fn parse_mthd(data: &[u8]) -> (MIDIFormat, u16, u16) {

    (match u16::from_be_bytes(data[0..2].try_into().expect("Error reading midi file.")) {
            0 => MIDIFormat::SingleTrack,
            1 => MIDIFormat::MultiTrack,
            2 => MIDIFormat::MultiSong,
            _ => panic!("Not a supported MIDI format."),
        },
        u16::from_be_bytes(data[2..4].try_into().expect("Error reading midi file.")),
        u16::from_be_bytes(data[4..6].try_into().expect("Error reading midi file.")),
    )
}

pub fn read_midi_file(path: &str) -> Result<MIDIFile, &'static str> {
    let data = fs::read(path)
        .expect(concat!("Can not read file ", stringify!(path)));

    assert!(&data.starts_with(b"MThd"), "Invaild midi file. MThd expected.");

    // Parse MThd Chunk
    let (format, track_num, division) = parse_mthd(&data[8..14]);
    let mut midi_file = MIDIFile {
        format,
        track_num,
        division,
        track: Vec::new(),
    };

    let mut offset: usize = 14;

    for _i in 0..midi_file.track_num {
        let chunk_length = u32::from_be_bytes(data[offset+4..offset+8].try_into().expect("Invaild chunk!")) as usize;

        if (data[offset..]).starts_with(b"MTrk") {
            let message_iter = MIDIMessageIter::from_bytes(&data[offset+8..offset+8+chunk_length], chunk_length);
            midi_file.track.push(MIDITrack {
                message: message_iter.into_iter().collect(),
            });
        }
        // Skip unknown chunks
        offset += 8 + chunk_length;
    }

    Ok(midi_file)
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_midi_head() {
        let mf = read_midi_file("tests/tiny.mid").expect("Read midi failed.");

        assert!(mf.format == MIDIFormat::MultiTrack);
        println!("{:?}", mf.track_num);
        println!("{:?}", mf.division);
        for t in mf.track {
            for m in t.message {
                if m.status == EventStatus::NoteOn {
                    println!("{:?}: {:?}", m.time, m.data);
                }
                if m.status == EventStatus::Meta {
                    if m.meta_type().unwrap() == MetaStatus::SetTempo {
                        println!("tempo {:?}", m.tempo_change().unwrap());
                    }
                }
            }
        }
        // assert!(mf.track_num == 18);
        // assert!(mf.tick_per_quarter == 960);
    }
}

