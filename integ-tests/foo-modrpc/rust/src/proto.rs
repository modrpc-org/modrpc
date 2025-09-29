use core::convert::TryFrom;
use mproto::{BaseLen, Compatible, Decode, DecodeCursor, DecodeError, DecodeResult, Encode, EncodeCursor, Lazy, Owned};

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct FooInitState {
    pub fooness: std_modrpc::PropertyInitState<u64>,
}

pub struct FooInitStateLazy<'a> {
    buffer: &'a [u8],
    offset: usize,
}

pub struct FooInitStateGen<
    Fooness: Encode + Compatible<std_modrpc::PropertyInitState<u64>>,
> {
    pub fooness: Fooness,
}

impl<
    Fooness: Encode + Compatible<std_modrpc::PropertyInitState<u64>>
> Compatible<FooInitState> for FooInitStateGen<Fooness> { }
impl<
    Fooness: Encode + Compatible<std_modrpc::PropertyInitState<u64>>
> Compatible<FooInitStateGen<Fooness>> for FooInitState { }

impl<
    Fooness: Encode + Compatible<std_modrpc::PropertyInitState<u64>>,
> BaseLen for FooInitStateGen<Fooness> {
    const BASE_LEN: usize = Fooness::BASE_LEN;
}

impl<
    Fooness: Encode + Compatible<std_modrpc::PropertyInitState<u64>>,
> Encode for FooInitStateGen<Fooness> {
    fn scratch_len(&self) -> usize {
        self.fooness.scratch_len()
    }

    fn encode(&self, cursor: &mut EncodeCursor) {
        self.fooness.encode(cursor);
    }
}

impl Owned for FooInitState {
    type Lazy<'a> = FooInitStateLazy<'a>;

    fn lazy_to_owned(lazy: Self::Lazy<'_>) -> DecodeResult<Self> {
        TryFrom::try_from(lazy)
    }
}

impl<'a> Lazy<'a> for FooInitStateLazy<'a> {
    type Owned = FooInitState;
}

impl<'a> Compatible<FooInitStateLazy<'a>> for FooInitState { }
impl<'a> Compatible<FooInitState> for FooInitStateLazy<'a> { }

impl<'a> FooInitStateLazy<'a> {

    pub fn fooness(&self) -> DecodeResult<std_modrpc::PropertyInitStateLazy<'a, u64>> {
        Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 0))
    }
}

impl BaseLen for FooInitState {
    const BASE_LEN: usize = 8;
}

impl Encode for FooInitState {
    fn scratch_len(&self) -> usize {
        self.fooness.scratch_len()
    }

    fn encode(&self, cursor: &mut EncodeCursor) {
        self.fooness.encode(cursor);
    }
}

impl<'a> Decode<'a> for FooInitState {
    fn decode(cursor: &DecodeCursor<'a>) -> DecodeResult<Self> {
        let fooness = Decode::decode(cursor)?;

        Ok(FooInitState {
            fooness,
        })
    }
}

impl<'a> BaseLen for FooInitStateLazy<'a> {
    const BASE_LEN: usize = 8;
}

impl<'a> Encode for FooInitStateLazy<'a> {
    fn scratch_len(&self) -> usize {
        let fooness: std_modrpc::PropertyInitStateLazy<'a, u64> = Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 0)).unwrap();
        fooness.scratch_len()
    }

    fn encode(&self, cursor: &mut EncodeCursor) {
        let fooness: std_modrpc::PropertyInitStateLazy<'a, u64> = Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 0)).unwrap();
        fooness.encode(cursor);
    }
}

impl<'a> Decode<'a> for FooInitStateLazy<'a> {
    fn decode(cursor: &DecodeCursor<'a>) -> DecodeResult<Self> {
        let offset = cursor.offset();
        cursor.advance(Self::BASE_LEN);
        Ok(FooInitStateLazy {
            buffer: cursor.buffer(),
            offset,
        })
    }
}

impl<'a> TryFrom<FooInitStateLazy<'a>> for FooInitState {
    type Error = DecodeError;

    fn try_from(other: FooInitStateLazy<'a>) -> Result<Self, Self::Error> {
        let cursor = DecodeCursor::at_offset(other.buffer, other.offset);
        Decode::decode(&cursor)
    }
}

impl<'a> Copy for FooInitStateLazy<'a> { }

impl<'a> Clone for FooInitStateLazy<'a> {
    fn clone(&self) -> Self {
        Self {
            buffer: self.buffer,
            offset: self.offset,
        }
    }
}

impl<'a> core::fmt::Debug for FooInitStateLazy<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("FooInitStateLazy")
            .finish()
    }
}

impl<'a> PartialEq for FooInitStateLazy<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.fooness().unwrap() == other.fooness().unwrap()
    }
}

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Default)]
pub struct FooClientConfig {}

pub struct FooClientConfigLazy<'a> {
    buffer: &'a [u8],
    offset: usize,
}

pub struct FooClientConfigGen<> {}

impl<> Compatible<FooClientConfig> for FooClientConfigGen<> { }
impl<> Compatible<FooClientConfigGen<>> for FooClientConfig { }

impl<> BaseLen for FooClientConfigGen<> {
    const BASE_LEN: usize = 0;
}

impl<> Encode for FooClientConfigGen<> {
    fn scratch_len(&self) -> usize {
        0
    }

    fn encode(&self, _: &mut EncodeCursor) {}
}

impl Owned for FooClientConfig {
    type Lazy<'a> = FooClientConfigLazy<'a>;

    fn lazy_to_owned(lazy: Self::Lazy<'_>) -> DecodeResult<Self> {
        TryFrom::try_from(lazy)
    }
}

impl<'a> Lazy<'a> for FooClientConfigLazy<'a> {
    type Owned = FooClientConfig;
}

impl<'a> Compatible<FooClientConfigLazy<'a>> for FooClientConfig { }
impl<'a> Compatible<FooClientConfig> for FooClientConfigLazy<'a> { }

impl<'a> FooClientConfigLazy<'a> {}

impl BaseLen for FooClientConfig {
    const BASE_LEN: usize = 0;
}

impl Encode for FooClientConfig {
    fn scratch_len(&self) -> usize {
        0
    }

    fn encode(&self, _: &mut EncodeCursor) {}
}

impl<'a> Decode<'a> for FooClientConfig {
    fn decode(_: &DecodeCursor<'a>) -> DecodeResult<Self> {

        Ok(FooClientConfig {})
    }
}

impl<'a> BaseLen for FooClientConfigLazy<'a> {
    const BASE_LEN: usize = 0;
}

impl<'a> Encode for FooClientConfigLazy<'a> {
    fn scratch_len(&self) -> usize {
        0
    }

    fn encode(&self, _: &mut EncodeCursor) {}
}

impl<'a> Decode<'a> for FooClientConfigLazy<'a> {
    fn decode(cursor: &DecodeCursor<'a>) -> DecodeResult<Self> {
        let offset = cursor.offset();
        cursor.advance(Self::BASE_LEN);
        Ok(FooClientConfigLazy {
            buffer: cursor.buffer(),
            offset,
        })
    }
}

impl<'a> TryFrom<FooClientConfigLazy<'a>> for FooClientConfig {
    type Error = DecodeError;

    fn try_from(other: FooClientConfigLazy<'a>) -> Result<Self, Self::Error> {
        let cursor = DecodeCursor::at_offset(other.buffer, other.offset);
        Decode::decode(&cursor)
    }
}

impl<'a> Copy for FooClientConfigLazy<'a> { }

impl<'a> Clone for FooClientConfigLazy<'a> {
    fn clone(&self) -> Self {
        Self {
            buffer: self.buffer,
            offset: self.offset,
        }
    }
}

impl<'a> core::fmt::Debug for FooClientConfigLazy<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("FooClientConfigLazy")
            .finish()
    }
}

impl<'a> PartialEq for FooClientConfigLazy<'a> {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Default)]
pub struct FooServerConfig {}

pub struct FooServerConfigLazy<'a> {
    buffer: &'a [u8],
    offset: usize,
}

pub struct FooServerConfigGen<> {}

impl<> Compatible<FooServerConfig> for FooServerConfigGen<> { }
impl<> Compatible<FooServerConfigGen<>> for FooServerConfig { }

impl<> BaseLen for FooServerConfigGen<> {
    const BASE_LEN: usize = 0;
}

impl<> Encode for FooServerConfigGen<> {
    fn scratch_len(&self) -> usize {
        0
    }

    fn encode(&self, _: &mut EncodeCursor) {}
}

impl Owned for FooServerConfig {
    type Lazy<'a> = FooServerConfigLazy<'a>;

    fn lazy_to_owned(lazy: Self::Lazy<'_>) -> DecodeResult<Self> {
        TryFrom::try_from(lazy)
    }
}

impl<'a> Lazy<'a> for FooServerConfigLazy<'a> {
    type Owned = FooServerConfig;
}

impl<'a> Compatible<FooServerConfigLazy<'a>> for FooServerConfig { }
impl<'a> Compatible<FooServerConfig> for FooServerConfigLazy<'a> { }

impl<'a> FooServerConfigLazy<'a> {}

impl BaseLen for FooServerConfig {
    const BASE_LEN: usize = 0;
}

impl Encode for FooServerConfig {
    fn scratch_len(&self) -> usize {
        0
    }

    fn encode(&self, _: &mut EncodeCursor) {}
}

impl<'a> Decode<'a> for FooServerConfig {
    fn decode(_: &DecodeCursor<'a>) -> DecodeResult<Self> {

        Ok(FooServerConfig {})
    }
}

impl<'a> BaseLen for FooServerConfigLazy<'a> {
    const BASE_LEN: usize = 0;
}

impl<'a> Encode for FooServerConfigLazy<'a> {
    fn scratch_len(&self) -> usize {
        0
    }

    fn encode(&self, _: &mut EncodeCursor) {}
}

impl<'a> Decode<'a> for FooServerConfigLazy<'a> {
    fn decode(cursor: &DecodeCursor<'a>) -> DecodeResult<Self> {
        let offset = cursor.offset();
        cursor.advance(Self::BASE_LEN);
        Ok(FooServerConfigLazy {
            buffer: cursor.buffer(),
            offset,
        })
    }
}

impl<'a> TryFrom<FooServerConfigLazy<'a>> for FooServerConfig {
    type Error = DecodeError;

    fn try_from(other: FooServerConfigLazy<'a>) -> Result<Self, Self::Error> {
        let cursor = DecodeCursor::at_offset(other.buffer, other.offset);
        Decode::decode(&cursor)
    }
}

impl<'a> Copy for FooServerConfigLazy<'a> { }

impl<'a> Clone for FooServerConfigLazy<'a> {
    fn clone(&self) -> Self {
        Self {
            buffer: self.buffer,
            offset: self.offset,
        }
    }
}

impl<'a> core::fmt::Debug for FooServerConfigLazy<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("FooServerConfigLazy")
            .finish()
    }
}

impl<'a> PartialEq for FooServerConfigLazy<'a> {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}
