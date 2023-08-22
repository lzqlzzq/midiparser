mod io;
mod message;
mod util;
mod sequence;

use pyo3::prelude::*;
pub use crate::io::{MIDIFile};
pub use crate::message::{EventStatus, MIDIMessage, MIDIFormat, MetaStatus};
pub use crate::util::{read_variable_length};
pub use crate::sequence::*;

#[pymodule]
fn midiparse_core(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<Sequence>()?;
    m.add_class::<Track>()?;
    m.add_class::<TrackTrans>()?;
    m.add_class::<Note>()?;
    m.add_class::<TimeSignature>()?;
    m.add_class::<KeySignature>()?;
    m.add_class::<Tempo>()?;
    Ok(())
}