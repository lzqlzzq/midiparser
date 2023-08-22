import numpy as np
import numba as nb
from typing import Optional
from .midiparse_core import Sequence, Track, TrackTrans


@nb.njit(nogil=True, cache=True)
def _to_pianoroll(piano, pitch, start, end):
    for p, s, e in zip(pitch, start, end):
        piano[0, p, s:e] = 1
        piano[1, p, s] = 1


def track2pianoroll(track: Track, max_len: Optional[int] = None, quantize: int = 24) -> np.ndarray:
    track: TrackTrans = track.transpose()
    pitch = np.asarray(track.pitch, dtype=np.uint8)
    start = np.asarray(track.start, dtype=np.float32)
    duration = np.asarray(track.duration, dtype=np.float32)
    start = (start * quantize + 0.5).astype(np.uint32)
    end = start + (duration * quantize + 0.5).astype(np.uint32)
    length = max_len if max_len is not None else end[-1]
    piano = np.zeros((2, 128, length), dtype=np.uint8)
    _to_pianoroll(piano, pitch, start, end)
    return piano


def seq2pianoroll(seq: Sequence, max_len: Optional[int] = None, quantize: int = 24) -> np.ndarray:
    end = int(max(track.notes[-1].end() for track in seq.tracks) * quantize + 0.5)
    length = max_len if max_len is not None else end
    assert length >= end
    return np.stack([
        track2pianoroll(track, max_len=length, quantize=quantize)
        for track in seq.tracks
    ])
