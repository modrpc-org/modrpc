use core::convert::TryFrom;
use mproto::{BaseLen, Compatible, Decode, DecodeCursor, DecodeError, DecodeResult, Encode, EncodeCursor, Lazy, Owned};

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Default)]
pub struct P2pBenchmarkInitState {}

pub struct P2pBenchmarkInitStateLazy<'a> {
    buffer: &'a [u8],
    offset: usize,
}

pub struct P2pBenchmarkInitStateGen<> {}

impl<> Compatible<P2pBenchmarkInitState> for P2pBenchmarkInitStateGen<> { }
impl<> Compatible<P2pBenchmarkInitStateGen<>> for P2pBenchmarkInitState { }

impl<> BaseLen for P2pBenchmarkInitStateGen<> {
    const BASE_LEN: usize = 0;
}

impl<> Encode for P2pBenchmarkInitStateGen<> {
    fn scratch_len(&self) -> usize {
        0
    }

    fn encode(&self, _: &mut EncodeCursor) {}
}

impl Owned for P2pBenchmarkInitState {
    type Lazy<'a> = P2pBenchmarkInitStateLazy<'a>;

    fn lazy_to_owned(lazy: Self::Lazy<'_>) -> DecodeResult<Self> {
        TryFrom::try_from(lazy)
    }
}

impl<'a> Lazy<'a> for P2pBenchmarkInitStateLazy<'a> {
    type Owned = P2pBenchmarkInitState;
}

impl<'a> Compatible<P2pBenchmarkInitStateLazy<'a>> for P2pBenchmarkInitStateLazy<'a> { }
impl<'a> Compatible<P2pBenchmarkInitStateLazy<'a>> for P2pBenchmarkInitState { }
impl Compatible<P2pBenchmarkInitState> for P2pBenchmarkInitState { }
impl<'a> Compatible<P2pBenchmarkInitState> for P2pBenchmarkInitStateLazy<'a> { }

impl<'a> P2pBenchmarkInitStateLazy<'a> {}

impl BaseLen for P2pBenchmarkInitState {
    const BASE_LEN: usize = 0;
}

impl Encode for P2pBenchmarkInitState {
    fn scratch_len(&self) -> usize {
        0
    }

    fn encode(&self, _: &mut EncodeCursor) {}
}

impl<'a> Decode<'a> for P2pBenchmarkInitState {
    fn decode(_: &DecodeCursor<'a>) -> DecodeResult<Self> {

        Ok(P2pBenchmarkInitState {})
    }
}

impl<'a> BaseLen for P2pBenchmarkInitStateLazy<'a> {
    const BASE_LEN: usize = 0;
}

impl<'a> Encode for P2pBenchmarkInitStateLazy<'a> {
    fn scratch_len(&self) -> usize {
        0
    }

    fn encode(&self, _: &mut EncodeCursor) {}
}

impl<'a> Decode<'a> for P2pBenchmarkInitStateLazy<'a> {
    fn decode(cursor: &DecodeCursor<'a>) -> DecodeResult<Self> {
        let offset = cursor.offset();
        cursor.advance(Self::BASE_LEN);
        Ok(P2pBenchmarkInitStateLazy {
            buffer: cursor.buffer(),
            offset,
        })
    }
}

impl<'a> TryFrom<P2pBenchmarkInitStateLazy<'a>> for P2pBenchmarkInitState {
    type Error = DecodeError;

    fn try_from(other: P2pBenchmarkInitStateLazy<'a>) -> Result<Self, Self::Error> {
        let cursor = DecodeCursor::at_offset(other.buffer, other.offset);
        Decode::decode(&cursor)
    }
}

impl<'a> Copy for P2pBenchmarkInitStateLazy<'a> { }

impl<'a> Clone for P2pBenchmarkInitStateLazy<'a> {
    fn clone(&self) -> Self {
        Self {
            buffer: self.buffer,
            offset: self.offset,
        }
    }
}

impl<'a> core::fmt::Debug for P2pBenchmarkInitStateLazy<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("P2pBenchmarkInitStateLazy")
            .finish()
    }
}

impl<'a> PartialEq for P2pBenchmarkInitStateLazy<'a> {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Default)]
pub struct P2pBenchmarkClientConfig {}

pub struct P2pBenchmarkClientConfigLazy<'a> {
    buffer: &'a [u8],
    offset: usize,
}

pub struct P2pBenchmarkClientConfigGen<> {}

impl<> Compatible<P2pBenchmarkClientConfig> for P2pBenchmarkClientConfigGen<> { }
impl<> Compatible<P2pBenchmarkClientConfigGen<>> for P2pBenchmarkClientConfig { }

impl<> BaseLen for P2pBenchmarkClientConfigGen<> {
    const BASE_LEN: usize = 0;
}

impl<> Encode for P2pBenchmarkClientConfigGen<> {
    fn scratch_len(&self) -> usize {
        0
    }

    fn encode(&self, _: &mut EncodeCursor) {}
}

impl Owned for P2pBenchmarkClientConfig {
    type Lazy<'a> = P2pBenchmarkClientConfigLazy<'a>;

    fn lazy_to_owned(lazy: Self::Lazy<'_>) -> DecodeResult<Self> {
        TryFrom::try_from(lazy)
    }
}

impl<'a> Lazy<'a> for P2pBenchmarkClientConfigLazy<'a> {
    type Owned = P2pBenchmarkClientConfig;
}

impl<'a> Compatible<P2pBenchmarkClientConfigLazy<'a>> for P2pBenchmarkClientConfigLazy<'a> { }
impl<'a> Compatible<P2pBenchmarkClientConfigLazy<'a>> for P2pBenchmarkClientConfig { }
impl Compatible<P2pBenchmarkClientConfig> for P2pBenchmarkClientConfig { }
impl<'a> Compatible<P2pBenchmarkClientConfig> for P2pBenchmarkClientConfigLazy<'a> { }

impl<'a> P2pBenchmarkClientConfigLazy<'a> {}

impl BaseLen for P2pBenchmarkClientConfig {
    const BASE_LEN: usize = 0;
}

impl Encode for P2pBenchmarkClientConfig {
    fn scratch_len(&self) -> usize {
        0
    }

    fn encode(&self, _: &mut EncodeCursor) {}
}

impl<'a> Decode<'a> for P2pBenchmarkClientConfig {
    fn decode(_: &DecodeCursor<'a>) -> DecodeResult<Self> {

        Ok(P2pBenchmarkClientConfig {})
    }
}

impl<'a> BaseLen for P2pBenchmarkClientConfigLazy<'a> {
    const BASE_LEN: usize = 0;
}

impl<'a> Encode for P2pBenchmarkClientConfigLazy<'a> {
    fn scratch_len(&self) -> usize {
        0
    }

    fn encode(&self, _: &mut EncodeCursor) {}
}

impl<'a> Decode<'a> for P2pBenchmarkClientConfigLazy<'a> {
    fn decode(cursor: &DecodeCursor<'a>) -> DecodeResult<Self> {
        let offset = cursor.offset();
        cursor.advance(Self::BASE_LEN);
        Ok(P2pBenchmarkClientConfigLazy {
            buffer: cursor.buffer(),
            offset,
        })
    }
}

impl<'a> TryFrom<P2pBenchmarkClientConfigLazy<'a>> for P2pBenchmarkClientConfig {
    type Error = DecodeError;

    fn try_from(other: P2pBenchmarkClientConfigLazy<'a>) -> Result<Self, Self::Error> {
        let cursor = DecodeCursor::at_offset(other.buffer, other.offset);
        Decode::decode(&cursor)
    }
}

impl<'a> Copy for P2pBenchmarkClientConfigLazy<'a> { }

impl<'a> Clone for P2pBenchmarkClientConfigLazy<'a> {
    fn clone(&self) -> Self {
        Self {
            buffer: self.buffer,
            offset: self.offset,
        }
    }
}

impl<'a> core::fmt::Debug for P2pBenchmarkClientConfigLazy<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("P2pBenchmarkClientConfigLazy")
            .finish()
    }
}

impl<'a> PartialEq for P2pBenchmarkClientConfigLazy<'a> {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Default)]
pub struct P2pBenchmarkServerConfig {}

pub struct P2pBenchmarkServerConfigLazy<'a> {
    buffer: &'a [u8],
    offset: usize,
}

pub struct P2pBenchmarkServerConfigGen<> {}

impl<> Compatible<P2pBenchmarkServerConfig> for P2pBenchmarkServerConfigGen<> { }
impl<> Compatible<P2pBenchmarkServerConfigGen<>> for P2pBenchmarkServerConfig { }

impl<> BaseLen for P2pBenchmarkServerConfigGen<> {
    const BASE_LEN: usize = 0;
}

impl<> Encode for P2pBenchmarkServerConfigGen<> {
    fn scratch_len(&self) -> usize {
        0
    }

    fn encode(&self, _: &mut EncodeCursor) {}
}

impl Owned for P2pBenchmarkServerConfig {
    type Lazy<'a> = P2pBenchmarkServerConfigLazy<'a>;

    fn lazy_to_owned(lazy: Self::Lazy<'_>) -> DecodeResult<Self> {
        TryFrom::try_from(lazy)
    }
}

impl<'a> Lazy<'a> for P2pBenchmarkServerConfigLazy<'a> {
    type Owned = P2pBenchmarkServerConfig;
}

impl<'a> Compatible<P2pBenchmarkServerConfigLazy<'a>> for P2pBenchmarkServerConfigLazy<'a> { }
impl<'a> Compatible<P2pBenchmarkServerConfigLazy<'a>> for P2pBenchmarkServerConfig { }
impl Compatible<P2pBenchmarkServerConfig> for P2pBenchmarkServerConfig { }
impl<'a> Compatible<P2pBenchmarkServerConfig> for P2pBenchmarkServerConfigLazy<'a> { }

impl<'a> P2pBenchmarkServerConfigLazy<'a> {}

impl BaseLen for P2pBenchmarkServerConfig {
    const BASE_LEN: usize = 0;
}

impl Encode for P2pBenchmarkServerConfig {
    fn scratch_len(&self) -> usize {
        0
    }

    fn encode(&self, _: &mut EncodeCursor) {}
}

impl<'a> Decode<'a> for P2pBenchmarkServerConfig {
    fn decode(_: &DecodeCursor<'a>) -> DecodeResult<Self> {

        Ok(P2pBenchmarkServerConfig {})
    }
}

impl<'a> BaseLen for P2pBenchmarkServerConfigLazy<'a> {
    const BASE_LEN: usize = 0;
}

impl<'a> Encode for P2pBenchmarkServerConfigLazy<'a> {
    fn scratch_len(&self) -> usize {
        0
    }

    fn encode(&self, _: &mut EncodeCursor) {}
}

impl<'a> Decode<'a> for P2pBenchmarkServerConfigLazy<'a> {
    fn decode(cursor: &DecodeCursor<'a>) -> DecodeResult<Self> {
        let offset = cursor.offset();
        cursor.advance(Self::BASE_LEN);
        Ok(P2pBenchmarkServerConfigLazy {
            buffer: cursor.buffer(),
            offset,
        })
    }
}

impl<'a> TryFrom<P2pBenchmarkServerConfigLazy<'a>> for P2pBenchmarkServerConfig {
    type Error = DecodeError;

    fn try_from(other: P2pBenchmarkServerConfigLazy<'a>) -> Result<Self, Self::Error> {
        let cursor = DecodeCursor::at_offset(other.buffer, other.offset);
        Decode::decode(&cursor)
    }
}

impl<'a> Copy for P2pBenchmarkServerConfigLazy<'a> { }

impl<'a> Clone for P2pBenchmarkServerConfigLazy<'a> {
    fn clone(&self) -> Self {
        Self {
            buffer: self.buffer,
            offset: self.offset,
        }
    }
}

impl<'a> core::fmt::Debug for P2pBenchmarkServerConfigLazy<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("P2pBenchmarkServerConfigLazy")
            .finish()
    }
}

impl<'a> PartialEq for P2pBenchmarkServerConfigLazy<'a> {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}
