#[derive(Debug, Clone)]
pub enum MIDIMessage {
    // 二者栈上内存占用一致，且都存在enum的成员
    // 经过编译优化后，这层enum应该不会增加内存开销
    Event(Event),
    Meta(Meta),
}

const EVENT_DATA_LEN: usize = 8;

#[derive(Debug, Copy, Clone)]
pub struct Event {
    pub time: u32,
    pub status: EventStatus,
    pub data: [u8; EVENT_DATA_LEN], // 空间足够，保持与Meta内存占用对齐
} // 有 Copy，默认逐位拷贝而不是move

#[derive(Debug, Clone)]
pub struct Meta {
    pub time: u32,
    pub status: MetaStatus,
    pub data: Box<[u8]>, // Box为胖指针，大小为16字节
} // Box，存在堆上空间，无法Copy, 实现Clone, 支持深拷贝，默认move

#[derive(PartialEq, Copy, Clone, Debug)]
pub enum MIDIFormat {
    SingleTrack = 0,
    MultiTrack = 1,
    MultiSong = 2,
}

#[derive(PartialEq, Copy, Clone, Debug)]
pub enum EventStatus {
    // Channel Voice Messages
    NoteOff = 0x80,
    NoteOn = 0x90,
    PolyphonicAfterTouch = 0xA0,
    ControlChange = 0xB0,
    ProgramChange = 0xC0,

    ChannelAfterTouch = 0xD0,
    PitchBend = 0xE0,

    // System Common Messages
    SysExStart = 0xF0,
    SongPositionPointer = 0xF2,
    SongSelect = 0xF3,
    TuneRequest = 0xF6,
    SysExEnd = 0xF7,
    TimingClock = 0xF8,
    StartSequence = 0xFA,
    ContinueSequence = 0xFB,
    StopSequence = 0xFC,
    ActiveSensing = 0xFE,
    // Meta Messages
    Meta = 0xFF,
}

#[derive(PartialEq, Copy, Clone, Debug)]
pub enum MetaStatus {
    SequenceNumber = 0x00,
    Text = 0x01,
    CopyrightNote = 0x02,
    TrackName = 0x03,
    InstrumentName = 0x04,
    Lyric = 0x05,
    Marker = 0x06,
    CuePoint = 0x07,
    MIDIChannelPrefix = 0x20,
    EndOfTrack = 0x2F,
    SetTempo = 0x51,
    SMPTEOffset = 0x54,
    TimeSignature = 0x58,
    KeySignature = 0x59,
    SequencerSpecificMeta = 0x7F,
    Unknown,
}

impl EventStatus {
    pub fn from_status_code(status: u8) -> (EventStatus, i8) {
        match status {
            0x80..=0x8F => (EventStatus::NoteOff, 3),
            0x90..=0x9F => (EventStatus::NoteOn, 3),
            0xA0..=0xAF => (EventStatus::PolyphonicAfterTouch, 3),
            0xB0..=0xBF => (EventStatus::ControlChange, 3),
            0xC0..=0xCF => (EventStatus::ProgramChange, 2),
            0xD0..=0xDF => (EventStatus::ChannelAfterTouch, 2),
            0xE0..=0xEF => (EventStatus::PitchBend, 3),
            0xF0 => (EventStatus::SysExStart, -1),
            0xF2 => (EventStatus::SongPositionPointer, 3),
            0xF3 => (EventStatus::SongSelect, 2),
            0xF6 => (EventStatus::TuneRequest, 1),
            0xF7 => (EventStatus::SysExEnd, 1),
            0xF8 => (EventStatus::TimingClock, 1),
            0xFA => (EventStatus::StartSequence, 1),
            0xFB => (EventStatus::ContinueSequence, 1),
            0xFC => (EventStatus::StopSequence, 1),
            0xFE => (EventStatus::ActiveSensing, 1),
            0xFF => (EventStatus::Meta, -1),
            _ => panic!("Event status code {:?} not implemented!", status),
        }
    }
}

impl MetaStatus {
    pub fn from_status_code(status: u8) -> MetaStatus {
        match status {
            0x00 => MetaStatus::SequenceNumber,
            0x01 => MetaStatus::Text,
            0x02 => MetaStatus::CopyrightNote,
            0x03 => MetaStatus::TrackName,
            0x04 => MetaStatus::InstrumentName,
            0x05 => MetaStatus::Lyric,
            0x06 => MetaStatus::Marker,
            0x07 => MetaStatus::CuePoint,
            0x20 => MetaStatus::MIDIChannelPrefix,
            0x2F => MetaStatus::EndOfTrack,
            0x51 => MetaStatus::SetTempo,
            0x54 => MetaStatus::SMPTEOffset,
            0x58 => MetaStatus::TimeSignature,
            0x59 => MetaStatus::KeySignature,
            0x7F => MetaStatus::SequencerSpecificMeta,
            // _ => panic!("Meta status code {:?} not implemented!", status),
            _ => MetaStatus::Unknown
        }
    }
}

impl MIDIMessage {
    #[inline(always)]
    pub fn new_event(time: u32, status_code: u8, data: &[u8]) -> Self {
        let (status, event_len) = EventStatus::from_status_code(status_code);
        assert_ne!(status, EventStatus::Meta, "Can't take a meta msg as a common msg!");
        let event_len = event_len as usize;
        let mut arr = [0; EVENT_DATA_LEN];
        arr[0] = status_code;
        arr[1..event_len].copy_from_slice(&data[..event_len - 1]);
        Self::Event(Event { time, status, data: arr })
    }

    #[inline(always)]
    pub fn new_meta(time: u32, status_code: u8, data: &[u8]) -> Self {
        assert_eq!(status_code, 0xFF, "Can't take other msg as a meta msg!");
        let status = MetaStatus::from_status_code(data[0]);
        let mut arr = vec![0; 1 + data.len()];
        arr[0] = status_code;
        arr[1..].copy_from_slice(data);
        Self::Meta(Meta { time, status, data: arr.into() })
    }
}

impl Event {
    #[inline(always)]
    pub fn channel(&self) -> Option<u8> {
        match self.status {
            EventStatus::NoteOff |
            EventStatus::NoteOn |
            EventStatus::PolyphonicAfterTouch |
            EventStatus::ControlChange |
            EventStatus::ProgramChange |
            EventStatus::ChannelAfterTouch |
            EventStatus::PitchBend => Some(self.data[0] & 0x0F),
            _ => None,
        }
    }

    #[inline(always)]
    pub fn key(&self) -> Option<u8> {
        match self.status {
            EventStatus::NoteOff |
            EventStatus::NoteOn |
            EventStatus::PolyphonicAfterTouch => Some(self.data[1]),
            _ => None,
        }
    }

    #[inline(always)]
    pub fn velocity(&self) -> Option<u8> {
        match self.status {
            EventStatus::NoteOff |
            EventStatus::NoteOn |
            EventStatus::PolyphonicAfterTouch => Some(self.data[2]),
            _ => None,
        }
    }

    #[inline(always)]
    pub fn control_change(&self) -> Option<(u8, u8)> {
        match self.status {
            EventStatus::ControlChange => Some((self.data[1], self.data[2])),
            _ => None
        }
    }

    #[inline(always)]
    pub fn program(&self) -> Option<u8> {
        match self.status {
            EventStatus::ProgramChange => Some(self.data[1]),
            _ => None
        }
    }
}

impl Meta {
    #[inline(always)]
    pub fn meta_value(&self) -> &[u8] {
        &self.data[3..]
    }

    #[inline(always)]
    pub fn tempo(&self) -> Option<u32> {
        match self.status {
            MetaStatus::SetTempo => {
                let mut tempo = [0; 4];
                tempo[1..].copy_from_slice(&self.data[3..6]);
                Some(u32::from_be_bytes(tempo))
            }
            _ => None
        }
    }

    #[inline(always)]
    pub fn key_signature(&self) -> Option<&'static str> {
        match self.status {
            MetaStatus::KeySignature => Some(
                if self.data[4] == 0 {
                    match self.data[3] as i8 {
                        -7i8 => "bC",
                        -6i8 => "bG",
                        -5i8 => "bD",
                        -4i8 => "bA",
                        -3i8 => "bE",
                        -2i8 => "bB",
                        -1i8 => "F",
                        0i8 => "C",
                        1i8 => "G",
                        2i8 => "D",
                        3i8 => "A",
                        4i8 => "E",
                        5i8 => "B",
                        6i8 => "#F",
                        7i8 => "#C",
                        _ => panic!("Not a valid key signature."),
                    }
                } else {
                    match self.data[3] as i8 {
                        -7i8 => "bc",
                        -6i8 => "bg",
                        -5i8 => "bd",
                        -4i8 => "ba",
                        -3i8 => "be",
                        -2i8 => "bb",
                        -1i8 => "f",
                        0i8 => "c",
                        1i8 => "g",
                        2i8 => "d",
                        3i8 => "a",
                        4i8 => "e",
                        5i8 => "b",
                        6i8 => "#f",
                        7i8 => "#c",
                        _ => panic!("Not a valid key signature."),
                    }
                }),
            _ => None,
        }
    }
    #[inline(always)]
    pub fn time_signature(&self) -> Option<(u8, u8, u8, u8)> {
        match self.status {
            MetaStatus::TimeSignature => Some((
                self.data[3],
                1 << self.data[4],
                self.data[5],
                self.data[6])),
            _ => None,
        }
    }
}