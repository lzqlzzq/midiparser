from typing import Tuple, List

class Sequence:
    def __init__(self, path: str): ...
    @property
    def tracks(self) -> List[Track]: ...

class TrackTrans:
    @property
    def pitch(self) -> List[int]: ...
    @property
    def start(self) -> List[float]: ...
    @property
    def duration(self) -> List[float]: ...
    @property
    def velocity(self) -> List[int]: ...

class Track:
    def transpose(self) -> TrackTrans: ...
    @property
    def notes(self) -> List[Note]: ...

class Note:
    __slots__ = ['pitch', 'start', 'duration', 'velocity']
    def __init__(self, pitch: int, start: float, duration: float, velocity: int): ...
    def end(self) -> float: ...
class Tempo:
    def __init__(self, time: float, qpm: int): ...

class TimeSignature:
    __slots__ = ['time', 'numerator', 'denominator']
    def __init__(self, time: float, numerator: int, denominator: int): ...

class KeySignature:
    __slots__ = ['time', 'key']
    def __init__(self, time: float, key: Tuple[bool, int]): ...