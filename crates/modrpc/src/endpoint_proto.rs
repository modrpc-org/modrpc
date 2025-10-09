use core::convert::TryFrom;
use mproto::{
    BaseLen, Compatible, Decode, DecodeCursor, DecodeError, DecodeResult, Encode, EncodeCursor,
    Lazy, Owned,
};

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct EndpointAddr {
    pub endpoint: u64,
}

pub struct EndpointAddrLazy<'a> {
    buffer: &'a [u8],
    offset: usize,
}

pub struct EndpointAddrGen {
    pub endpoint: u64,
}

impl Compatible<EndpointAddr> for EndpointAddrGen {}
impl Compatible<EndpointAddrGen> for EndpointAddr {}

impl BaseLen for EndpointAddrGen {
    const BASE_LEN: usize = 8;
}

impl Encode for EndpointAddrGen {
    fn scratch_len(&self) -> usize {
        self.endpoint.scratch_len()
    }

    fn encode(&self, cursor: &mut EncodeCursor) {
        self.endpoint.encode(cursor);
    }
}

impl Owned for EndpointAddr {
    type Lazy<'a> = EndpointAddrLazy<'a>;

    fn lazy_to_owned(lazy: Self::Lazy<'_>) -> DecodeResult<Self> {
        TryFrom::try_from(lazy)
    }
}

impl<'a> Lazy<'a> for EndpointAddrLazy<'a> {
    type Owned = EndpointAddr;
}

impl<'a> Compatible<EndpointAddrLazy<'a>> for EndpointAddrLazy<'a> {}
impl<'a> Compatible<EndpointAddrLazy<'a>> for EndpointAddr {}
impl Compatible<EndpointAddr> for EndpointAddr {}
impl<'a> Compatible<EndpointAddr> for EndpointAddrLazy<'a> {}

impl<'a> EndpointAddrLazy<'a> {
    pub fn endpoint(&self) -> DecodeResult<u64> {
        Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 0))
    }
}

impl BaseLen for EndpointAddr {
    const BASE_LEN: usize = 8;
}

impl Encode for EndpointAddr {
    fn scratch_len(&self) -> usize {
        self.endpoint.scratch_len()
    }

    fn encode(&self, cursor: &mut EncodeCursor) {
        self.endpoint.encode(cursor);
    }
}

impl<'a> Decode<'a> for EndpointAddr {
    fn decode(cursor: &DecodeCursor<'a>) -> DecodeResult<Self> {
        let endpoint = Decode::decode(cursor)?;

        Ok(EndpointAddr { endpoint })
    }
}

impl<'a> BaseLen for EndpointAddrLazy<'a> {
    const BASE_LEN: usize = 8;
}

impl<'a> Encode for EndpointAddrLazy<'a> {
    fn scratch_len(&self) -> usize {
        let endpoint: u64 =
            Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 0)).unwrap();
        endpoint.scratch_len()
    }

    fn encode(&self, cursor: &mut EncodeCursor) {
        let endpoint: u64 =
            Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 0)).unwrap();
        endpoint.encode(cursor);
    }
}

impl<'a> Decode<'a> for EndpointAddrLazy<'a> {
    fn decode(cursor: &DecodeCursor<'a>) -> DecodeResult<Self> {
        let offset = cursor.offset();
        cursor.advance(Self::BASE_LEN);
        Ok(EndpointAddrLazy {
            buffer: cursor.buffer(),
            offset,
        })
    }
}

impl<'a> TryFrom<EndpointAddrLazy<'a>> for EndpointAddr {
    type Error = DecodeError;

    fn try_from(other: EndpointAddrLazy<'a>) -> Result<Self, Self::Error> {
        let cursor = DecodeCursor::at_offset(other.buffer, other.offset);
        Decode::decode(&cursor)
    }
}

impl<'a> Copy for EndpointAddrLazy<'a> {}

impl<'a> Clone for EndpointAddrLazy<'a> {
    fn clone(&self) -> Self {
        Self {
            buffer: self.buffer,
            offset: self.offset,
        }
    }
}

impl<'a> core::fmt::Debug for EndpointAddrLazy<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("EndpointAddrLazy").finish()
    }
}

impl<'a> PartialEq for EndpointAddrLazy<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.endpoint().unwrap() == other.endpoint().unwrap()
    }
}

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct PacketBundle {
    pub channel_id: u32,
    pub length: u16,
}

pub struct PacketBundleLazy<'a> {
    buffer: &'a [u8],
    offset: usize,
}

pub struct PacketBundleGen {
    pub channel_id: u32,
    pub length: u16,
}

impl Compatible<PacketBundle> for PacketBundleGen {}
impl Compatible<PacketBundleGen> for PacketBundle {}

impl BaseLen for PacketBundleGen {
    const BASE_LEN: usize = 6;
}

impl Encode for PacketBundleGen {
    fn scratch_len(&self) -> usize {
        self.channel_id.scratch_len() + self.length.scratch_len()
    }

    fn encode(&self, cursor: &mut EncodeCursor) {
        self.channel_id.encode(cursor);
        self.length.encode(cursor);
    }
}

impl Owned for PacketBundle {
    type Lazy<'a> = PacketBundleLazy<'a>;

    fn lazy_to_owned(lazy: Self::Lazy<'_>) -> DecodeResult<Self> {
        TryFrom::try_from(lazy)
    }
}

impl<'a> Lazy<'a> for PacketBundleLazy<'a> {
    type Owned = PacketBundle;
}

impl<'a> Compatible<PacketBundleLazy<'a>> for PacketBundleLazy<'a> {}
impl<'a> Compatible<PacketBundleLazy<'a>> for PacketBundle {}
impl Compatible<PacketBundle> for PacketBundle {}
impl<'a> Compatible<PacketBundle> for PacketBundleLazy<'a> {}

impl<'a> PacketBundleLazy<'a> {
    pub fn channel_id(&self) -> DecodeResult<u32> {
        Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 0))
    }

    pub fn length(&self) -> DecodeResult<u16> {
        Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 4))
    }
}

impl BaseLen for PacketBundle {
    const BASE_LEN: usize = 6;
}

impl Encode for PacketBundle {
    fn scratch_len(&self) -> usize {
        self.channel_id.scratch_len() + self.length.scratch_len()
    }

    fn encode(&self, cursor: &mut EncodeCursor) {
        self.channel_id.encode(cursor);
        self.length.encode(cursor);
    }
}

impl<'a> Decode<'a> for PacketBundle {
    fn decode(cursor: &DecodeCursor<'a>) -> DecodeResult<Self> {
        let channel_id = Decode::decode(cursor)?;
        let length = Decode::decode(cursor)?;

        Ok(PacketBundle { channel_id, length })
    }
}

impl<'a> BaseLen for PacketBundleLazy<'a> {
    const BASE_LEN: usize = 6;
}

impl<'a> Encode for PacketBundleLazy<'a> {
    fn scratch_len(&self) -> usize {
        let channel_id: u32 =
            Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 0)).unwrap();
        let length: u16 =
            Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 4)).unwrap();
        channel_id.scratch_len() + length.scratch_len()
    }

    fn encode(&self, cursor: &mut EncodeCursor) {
        let channel_id: u32 =
            Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 0)).unwrap();
        let length: u16 =
            Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 4)).unwrap();
        channel_id.encode(cursor);
        length.encode(cursor);
    }
}

impl<'a> Decode<'a> for PacketBundleLazy<'a> {
    fn decode(cursor: &DecodeCursor<'a>) -> DecodeResult<Self> {
        let offset = cursor.offset();
        cursor.advance(Self::BASE_LEN);
        Ok(PacketBundleLazy {
            buffer: cursor.buffer(),
            offset,
        })
    }
}

impl<'a> TryFrom<PacketBundleLazy<'a>> for PacketBundle {
    type Error = DecodeError;

    fn try_from(other: PacketBundleLazy<'a>) -> Result<Self, Self::Error> {
        let cursor = DecodeCursor::at_offset(other.buffer, other.offset);
        Decode::decode(&cursor)
    }
}

impl<'a> Copy for PacketBundleLazy<'a> {}

impl<'a> Clone for PacketBundleLazy<'a> {
    fn clone(&self) -> Self {
        Self {
            buffer: self.buffer,
            offset: self.offset,
        }
    }
}

impl<'a> core::fmt::Debug for PacketBundleLazy<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("PacketBundleLazy").finish()
    }
}

impl<'a> PartialEq for PacketBundleLazy<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.channel_id().unwrap() == other.channel_id().unwrap()
            && self.length().unwrap() == other.length().unwrap()
    }
}

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct TransmitPacket {
    pub payload_length: u16,
    pub infra_id: u16,
    pub plane_id: u32,
    pub topic: u32,
    pub source: EndpointAddr,
}

pub struct TransmitPacketLazy<'a> {
    buffer: &'a [u8],
    offset: usize,
}

pub struct TransmitPacketGen<Source: Encode + Compatible<EndpointAddr>> {
    pub payload_length: u16,
    pub infra_id: u16,
    pub plane_id: u32,
    pub topic: u32,
    pub source: Source,
}

impl<Source: Encode + Compatible<EndpointAddr>> Compatible<TransmitPacket>
    for TransmitPacketGen<Source>
{
}
impl<Source: Encode + Compatible<EndpointAddr>> Compatible<TransmitPacketGen<Source>>
    for TransmitPacket
{
}

impl<Source: Encode + Compatible<EndpointAddr>> BaseLen for TransmitPacketGen<Source> {
    const BASE_LEN: usize = 12 + Source::BASE_LEN;
}

impl<Source: Encode + Compatible<EndpointAddr>> Encode for TransmitPacketGen<Source> {
    fn scratch_len(&self) -> usize {
        self.payload_length.scratch_len()
            + self.infra_id.scratch_len()
            + self.plane_id.scratch_len()
            + self.topic.scratch_len()
            + self.source.scratch_len()
    }

    fn encode(&self, cursor: &mut EncodeCursor) {
        self.payload_length.encode(cursor);
        self.infra_id.encode(cursor);
        self.plane_id.encode(cursor);
        self.topic.encode(cursor);
        self.source.encode(cursor);
    }
}

impl Owned for TransmitPacket {
    type Lazy<'a> = TransmitPacketLazy<'a>;

    fn lazy_to_owned(lazy: Self::Lazy<'_>) -> DecodeResult<Self> {
        TryFrom::try_from(lazy)
    }
}

impl<'a> Lazy<'a> for TransmitPacketLazy<'a> {
    type Owned = TransmitPacket;
}

impl<'a> Compatible<TransmitPacketLazy<'a>> for TransmitPacketLazy<'a> {}
impl<'a> Compatible<TransmitPacketLazy<'a>> for TransmitPacket {}
impl Compatible<TransmitPacket> for TransmitPacket {}
impl<'a> Compatible<TransmitPacket> for TransmitPacketLazy<'a> {}

impl<'a> TransmitPacketLazy<'a> {
    pub fn payload_length(&self) -> DecodeResult<u16> {
        Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 0))
    }

    pub fn infra_id(&self) -> DecodeResult<u16> {
        Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 2))
    }

    pub fn plane_id(&self) -> DecodeResult<u32> {
        Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 4))
    }

    pub fn topic(&self) -> DecodeResult<u32> {
        Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 8))
    }

    pub fn source(&self) -> DecodeResult<EndpointAddrLazy<'a>> {
        Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 12))
    }
}

impl BaseLen for TransmitPacket {
    const BASE_LEN: usize = 20;
}

impl Encode for TransmitPacket {
    fn scratch_len(&self) -> usize {
        self.payload_length.scratch_len()
            + self.infra_id.scratch_len()
            + self.plane_id.scratch_len()
            + self.topic.scratch_len()
            + self.source.scratch_len()
    }

    fn encode(&self, cursor: &mut EncodeCursor) {
        self.payload_length.encode(cursor);
        self.infra_id.encode(cursor);
        self.plane_id.encode(cursor);
        self.topic.encode(cursor);
        self.source.encode(cursor);
    }
}

impl<'a> Decode<'a> for TransmitPacket {
    fn decode(cursor: &DecodeCursor<'a>) -> DecodeResult<Self> {
        let payload_length = Decode::decode(cursor)?;
        let infra_id = Decode::decode(cursor)?;
        let plane_id = Decode::decode(cursor)?;
        let topic = Decode::decode(cursor)?;
        let source = Decode::decode(cursor)?;

        Ok(TransmitPacket {
            payload_length,
            infra_id,
            plane_id,
            topic,
            source,
        })
    }
}

impl<'a> BaseLen for TransmitPacketLazy<'a> {
    const BASE_LEN: usize = 20;
}

impl<'a> Encode for TransmitPacketLazy<'a> {
    fn scratch_len(&self) -> usize {
        let payload_length: u16 =
            Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 0)).unwrap();
        let infra_id: u16 =
            Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 2)).unwrap();
        let plane_id: u32 =
            Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 4)).unwrap();
        let topic: u32 =
            Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 8)).unwrap();
        let source: EndpointAddrLazy<'a> =
            Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 12)).unwrap();
        payload_length.scratch_len()
            + infra_id.scratch_len()
            + plane_id.scratch_len()
            + topic.scratch_len()
            + source.scratch_len()
    }

    fn encode(&self, cursor: &mut EncodeCursor) {
        let payload_length: u16 =
            Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 0)).unwrap();
        let infra_id: u16 =
            Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 2)).unwrap();
        let plane_id: u32 =
            Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 4)).unwrap();
        let topic: u32 =
            Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 8)).unwrap();
        let source: EndpointAddrLazy<'a> =
            Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 12)).unwrap();
        payload_length.encode(cursor);
        infra_id.encode(cursor);
        plane_id.encode(cursor);
        topic.encode(cursor);
        source.encode(cursor);
    }
}

impl<'a> Decode<'a> for TransmitPacketLazy<'a> {
    fn decode(cursor: &DecodeCursor<'a>) -> DecodeResult<Self> {
        let offset = cursor.offset();
        cursor.advance(Self::BASE_LEN);
        Ok(TransmitPacketLazy {
            buffer: cursor.buffer(),
            offset,
        })
    }
}

impl<'a> TryFrom<TransmitPacketLazy<'a>> for TransmitPacket {
    type Error = DecodeError;

    fn try_from(other: TransmitPacketLazy<'a>) -> Result<Self, Self::Error> {
        let cursor = DecodeCursor::at_offset(other.buffer, other.offset);
        Decode::decode(&cursor)
    }
}

impl<'a> Copy for TransmitPacketLazy<'a> {}

impl<'a> Clone for TransmitPacketLazy<'a> {
    fn clone(&self) -> Self {
        Self {
            buffer: self.buffer,
            offset: self.offset,
        }
    }
}

impl<'a> core::fmt::Debug for TransmitPacketLazy<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TransmitPacketLazy").finish()
    }
}

impl<'a> PartialEq for TransmitPacketLazy<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.payload_length().unwrap() == other.payload_length().unwrap()
            && self.infra_id().unwrap() == other.infra_id().unwrap()
            && self.plane_id().unwrap() == other.plane_id().unwrap()
            && self.topic().unwrap() == other.topic().unwrap()
            && self.source().unwrap() == other.source().unwrap()
    }
}

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct PlaneHandshake<T> {
    pub plane_id: u32,
    pub endpoint_addr: EndpointAddr,
    pub init: T,
}

pub struct PlaneHandshakeLazy<'a, T> {
    buffer: &'a [u8],
    offset: usize,
    _t: core::marker::PhantomData<T>,
}

pub struct PlaneHandshakeGen<TEndpointAddr: Encode + Compatible<EndpointAddr>, Init: Encode> {
    pub plane_id: u32,
    pub endpoint_addr: TEndpointAddr,
    pub init: Init,
}

impl<T: Owned, TEndpointAddr: Encode + Compatible<EndpointAddr>, Init: Encode + Compatible<T>>
    Compatible<PlaneHandshake<T>> for PlaneHandshakeGen<TEndpointAddr, Init>
{
}
impl<T: Owned, TEndpointAddr: Encode + Compatible<EndpointAddr>, Init: Encode + Compatible<T>>
    Compatible<PlaneHandshakeGen<TEndpointAddr, Init>> for PlaneHandshake<T>
{
}

impl<TEndpointAddr: Encode + Compatible<EndpointAddr>, Init: Encode> BaseLen
    for PlaneHandshakeGen<TEndpointAddr, Init>
{
    const BASE_LEN: usize = 4 + TEndpointAddr::BASE_LEN + Init::BASE_LEN;
}

impl<TEndpointAddr: Encode + Compatible<EndpointAddr>, Init: Encode> Encode
    for PlaneHandshakeGen<TEndpointAddr, Init>
{
    fn scratch_len(&self) -> usize {
        self.plane_id.scratch_len() + self.endpoint_addr.scratch_len() + self.init.scratch_len()
    }

    fn encode(&self, cursor: &mut EncodeCursor) {
        self.plane_id.encode(cursor);
        self.endpoint_addr.encode(cursor);
        self.init.encode(cursor);
    }
}

impl<T: Owned> Owned for PlaneHandshake<T> {
    type Lazy<'a> = PlaneHandshakeLazy<'a, T>;

    fn lazy_to_owned(lazy: Self::Lazy<'_>) -> DecodeResult<Self> {
        TryFrom::try_from(lazy)
    }
}

impl<'a, T: Owned> Lazy<'a> for PlaneHandshakeLazy<'a, T> {
    type Owned = PlaneHandshake<T>;
}

impl<'a, T: Owned> Compatible<PlaneHandshakeLazy<'a, T>> for PlaneHandshakeLazy<'a, T> {}
impl<'a, T: Owned> Compatible<PlaneHandshakeLazy<'a, T>> for PlaneHandshake<T> {}
impl<T: Owned> Compatible<PlaneHandshake<T>> for PlaneHandshake<T> {}
impl<'a, T: Owned> Compatible<PlaneHandshake<T>> for PlaneHandshakeLazy<'a, T> {}

impl<'a, T: Owned> PlaneHandshakeLazy<'a, T> {
    pub fn plane_id(&self) -> DecodeResult<u32> {
        Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 0))
    }

    pub fn endpoint_addr(&self) -> DecodeResult<EndpointAddrLazy<'a>> {
        Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 4))
    }

    pub fn init(&self) -> DecodeResult<T::Lazy<'a>> {
        Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 12))
    }
}

impl<T: BaseLen> BaseLen for PlaneHandshake<T> {
    const BASE_LEN: usize = 12 + T::BASE_LEN;
}

impl<T: Encode> Encode for PlaneHandshake<T> {
    fn scratch_len(&self) -> usize {
        self.plane_id.scratch_len() + self.endpoint_addr.scratch_len() + self.init.scratch_len()
    }

    fn encode(&self, cursor: &mut EncodeCursor) {
        self.plane_id.encode(cursor);
        self.endpoint_addr.encode(cursor);
        self.init.encode(cursor);
    }
}

impl<'a, T: Decode<'a>> Decode<'a> for PlaneHandshake<T> {
    fn decode(cursor: &DecodeCursor<'a>) -> DecodeResult<Self> {
        let plane_id = Decode::decode(cursor)?;
        let endpoint_addr = Decode::decode(cursor)?;
        let init = Decode::decode(cursor)?;

        Ok(PlaneHandshake {
            plane_id,
            endpoint_addr,
            init,
        })
    }
}

impl<'a, T: Owned> BaseLen for PlaneHandshakeLazy<'a, T> {
    const BASE_LEN: usize = 12 + T::BASE_LEN;
}

impl<'a, T: Owned> Encode for PlaneHandshakeLazy<'a, T> {
    fn scratch_len(&self) -> usize {
        let plane_id: u32 =
            Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 0)).unwrap();
        let endpoint_addr: EndpointAddrLazy<'a> =
            Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 4)).unwrap();
        let init: T::Lazy<'a> =
            Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 12)).unwrap();
        plane_id.scratch_len() + endpoint_addr.scratch_len() + init.scratch_len()
    }

    fn encode(&self, cursor: &mut EncodeCursor) {
        let plane_id: u32 =
            Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 0)).unwrap();
        let endpoint_addr: EndpointAddrLazy<'a> =
            Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 4)).unwrap();
        let init: T::Lazy<'a> =
            Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 12)).unwrap();
        plane_id.encode(cursor);
        endpoint_addr.encode(cursor);
        init.encode(cursor);
    }
}

impl<'a, T: Owned> Decode<'a> for PlaneHandshakeLazy<'a, T> {
    fn decode(cursor: &DecodeCursor<'a>) -> DecodeResult<Self> {
        let offset = cursor.offset();
        cursor.advance(Self::BASE_LEN);
        Ok(PlaneHandshakeLazy {
            buffer: cursor.buffer(),
            offset,
            _t: core::marker::PhantomData,
        })
    }
}

impl<'a, T: Owned> TryFrom<PlaneHandshakeLazy<'a, T>> for PlaneHandshake<T> {
    type Error = DecodeError;

    fn try_from(other: PlaneHandshakeLazy<'a, T>) -> Result<Self, Self::Error> {
        let cursor = DecodeCursor::at_offset(other.buffer, other.offset);
        Decode::decode(&cursor)
    }
}

impl<'a, T> Copy for PlaneHandshakeLazy<'a, T> {}

impl<'a, T> Clone for PlaneHandshakeLazy<'a, T> {
    fn clone(&self) -> Self {
        Self {
            buffer: self.buffer,
            offset: self.offset,
            _t: core::marker::PhantomData,
        }
    }
}

impl<'a, T> core::fmt::Debug for PlaneHandshakeLazy<'a, T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("PlaneHandshakeLazy").finish()
    }
}

impl<'a, T: Owned> PartialEq for PlaneHandshakeLazy<'a, T> {
    fn eq(&self, other: &Self) -> bool {
        self.plane_id().unwrap() == other.plane_id().unwrap()
            && self.endpoint_addr().unwrap() == other.endpoint_addr().unwrap()
            && self.init().unwrap() == other.init().unwrap()
    }
}
