use std::fs;
use std::str;
use crate::message::{ EventStatus, MIDIMessage, MIDIFormat, MetaStatus };
use crate::util:: { read_variable_length };

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

fn parse_mtrk(data: &[u8], bytes: usize) -> Result<MIDITrack, &'static str> {
    let mut track = MIDITrack { message: Vec::new(), };
    let mut byte_offset: usize = 0;
    let mut tick_offset: u32 = 0;

    let mut last_status = EventStatus::Meta;
    let mut last_event_len: usize = 0;
    let mut last_status_code: u8 = 0;

    while byte_offset < bytes {
        let (bytes, value) = read_variable_length(&data[byte_offset..byte_offset+4].try_into().expect("Reading variable length error."));
        byte_offset += bytes as usize;
        tick_offset += value as u32;
        let this_status = &data[byte_offset];

        let (event_status, message_data, length_to_offset) = match &this_status {
            // Running status of MIDI Messages has original length - 1.
            0x00..=0x7F => {
                let mut message = vec![last_status_code];
                message.extend_from_slice(&data[byte_offset..byte_offset+last_event_len-1]);
                (last_status.clone(), message, last_event_len - 1)
            },
            // MIDI Messages and SysEx Messages has determinated length.
            0x80..=0xFE => {
                last_status_code = this_status.clone();
                let (status, event_len) = EventStatus::from_status_code(&data[byte_offset]);
                (last_status, last_event_len) = (status.clone(), event_len as usize);
                (status, Vec::from(&data[byte_offset..byte_offset+(event_len as usize)]), event_len as usize)
            },
            // Meta Messages has variable length.
            0xFF => {
                let (meta_length_bytes, mut metalen) = read_variable_length(match data.get(byte_offset+2..byte_offset+6)
                {
                    Some(result) => result.try_into().unwrap(),
                    None => &[0u8, 0u8, 0u8, 0u8],
                });
                metalen += (meta_length_bytes as usize) + 2;
                (EventStatus::from_status_code(this_status).0, Vec::from(&data[byte_offset..byte_offset+metalen]), metalen)
            },
        };

        track.message.push(MIDIMessage {
            time: tick_offset,
            status: event_status,
            data: message_data,
        });

        byte_offset += length_to_offset;
    }

    Ok(track)
}

pub fn read_midi_file(path: &str) -> Result<MIDIFile, &'static str> {
    let data = fs::read(path)
        .expect(concat!("Can not read file ", stringify!(path)));

    assert!(&data.starts_with(b"MThd"), "Invaild midi file. MThd expected.");

    // Parse MThd Chunk
    let (format, track_num, division) = parse_mthd(&data[8..14]);
    let mut midi_file = MIDIFile {
        format: format,
        track_num: track_num,
        division: division,
        track: Vec::new(),
    };

    let mut offset: usize = 14;

    for _i in 0..midi_file.track_num {
        let chunk_length = u32::from_be_bytes(data[offset+4..offset+8].try_into().expect("Invaild chunk!")) as usize;

        if (&data[offset..]).starts_with(b"MTrk") {
            midi_file.track.push(parse_mtrk(&data[offset+8..offset+8+chunk_length], chunk_length).expect("Read chunk failed."));
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
        let mf = read_midi_file("tests/tiny2.mid").expect("Read midi failed.");

        assert!(mf.format == MIDIFormat::MultiTrack);
        println!("{:?}", mf.track_num);
        println!("{:?}", mf.division);
        for t in mf.track {
            for m in t.message {
                if(m.status == EventStatus::NoteOn) {
                    println!("{:?}: {:?}", m.time, m.data);
                }
                if((m.status == EventStatus::Meta) ) {
                    if(m.meta_type().unwrap() == MetaStatus::SetTempo) {
                        println!("tempo {:?}", m.tempo_change().unwrap());
                    }
                }
            }
        }
        // assert!(mf.track_num == 18);
        // assert!(mf.tick_per_quarter == 960);
    }
}

