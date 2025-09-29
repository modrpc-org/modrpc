use core::convert::TryFrom;
use mproto::{BaseLen, Compatible, Decode, DecodeCursor, DecodeError, DecodeResult, Encode, EncodeCursor, Lazy, Owned};

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct PropertyUpdate<T> {
    pub new_value: T,
}

pub struct PropertyUpdateLazy<'a, T> {
    buffer: &'a [u8],
    offset: usize,
    _t: core::marker::PhantomData<T>,
}

pub struct PropertyUpdateGen<
    NewValue: Encode,
> {
    pub new_value: NewValue,
}

impl<
    T: Owned,
    NewValue: Encode + Compatible<T>
> Compatible<PropertyUpdate<T>> for PropertyUpdateGen<NewValue> { }
impl<
    T: Owned,
    NewValue: Encode + Compatible<T>
> Compatible<PropertyUpdateGen<NewValue>> for PropertyUpdate<T> { }

impl<
    NewValue: Encode,
> BaseLen for PropertyUpdateGen<NewValue> {
    const BASE_LEN: usize = NewValue::BASE_LEN;
}

impl<
    NewValue: Encode,
> Encode for PropertyUpdateGen<NewValue> {
    fn scratch_len(&self) -> usize {
        self.new_value.scratch_len()
    }

    fn encode(&self, cursor: &mut EncodeCursor) {
        self.new_value.encode(cursor);
    }
}

impl<T: Owned> Owned for PropertyUpdate<T> {
    type Lazy<'a> = PropertyUpdateLazy<'a, T>;

    fn lazy_to_owned(lazy: Self::Lazy<'_>) -> DecodeResult<Self> {
        TryFrom::try_from(lazy)
    }
}

impl<'a, T: Owned> Lazy<'a> for PropertyUpdateLazy<'a, T> {
    type Owned = PropertyUpdate<T>;
}

impl<'a, T: Owned> Compatible<PropertyUpdateLazy<'a, T>> for PropertyUpdate<T> { }
impl<'a, T: Owned> Compatible<PropertyUpdate<T>> for PropertyUpdateLazy<'a, T> { }

impl<'a, T: Owned> PropertyUpdateLazy<'a, T> {

    pub fn new_value(&self) -> DecodeResult<T::Lazy<'a>> {
        Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 0))
    }
}

impl<T: BaseLen> BaseLen for PropertyUpdate<T> {
    const BASE_LEN: usize = T::BASE_LEN;
}

impl<T: Encode> Encode for PropertyUpdate<T> {
    fn scratch_len(&self) -> usize {
        self.new_value.scratch_len()
    }

    fn encode(&self, cursor: &mut EncodeCursor) {
        self.new_value.encode(cursor);
    }
}

impl<'a, T: Decode<'a>> Decode<'a> for PropertyUpdate<T> {
    fn decode(cursor: &DecodeCursor<'a>) -> DecodeResult<Self> {
        let new_value = Decode::decode(cursor)?;

        Ok(PropertyUpdate {
            new_value,
        })
    }
}

impl<'a, T: Owned> BaseLen for PropertyUpdateLazy<'a, T> {
    const BASE_LEN: usize = T::BASE_LEN;
}

impl<'a, T: Owned> Encode for PropertyUpdateLazy<'a, T> {
    fn scratch_len(&self) -> usize {
        let new_value: T::Lazy<'a> = Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 0)).unwrap();
        new_value.scratch_len()
    }

    fn encode(&self, cursor: &mut EncodeCursor) {
        let new_value: T::Lazy<'a> = Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 0)).unwrap();
        new_value.encode(cursor);
    }
}

impl<'a, T: Owned> Decode<'a> for PropertyUpdateLazy<'a, T> {
    fn decode(cursor: &DecodeCursor<'a>) -> DecodeResult<Self> {
        let offset = cursor.offset();
        cursor.advance(Self::BASE_LEN);
        Ok(PropertyUpdateLazy {
            buffer: cursor.buffer(),
            offset,
            _t: core::marker::PhantomData,
        })
    }
}

impl<'a, T: Owned> TryFrom<PropertyUpdateLazy<'a, T>> for PropertyUpdate<T> {
    type Error = DecodeError;

    fn try_from(other: PropertyUpdateLazy<'a, T>) -> Result<Self, Self::Error> {
        let cursor = DecodeCursor::at_offset(other.buffer, other.offset);
        Decode::decode(&cursor)
    }
}

impl<'a, T> Copy for PropertyUpdateLazy<'a, T> { }

impl<'a, T> Clone for PropertyUpdateLazy<'a, T> {
    fn clone(&self) -> Self {
        Self {
            buffer: self.buffer,
            offset: self.offset,
            _t: core::marker::PhantomData,
        }
    }
}

impl<'a, T> core::fmt::Debug for PropertyUpdateLazy<'a, T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("PropertyUpdateLazy")
            .finish()
    }
}

impl<'a, T: Owned> PartialEq for PropertyUpdateLazy<'a, T> {
    fn eq(&self, other: &Self) -> bool {
        self.new_value().unwrap() == other.new_value().unwrap()
    }
}

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct Request<T> {
    pub request_id: u32,
    pub worker: u16,
    pub payload: T,
}

pub struct RequestLazy<'a, T> {
    buffer: &'a [u8],
    offset: usize,
    _t: core::marker::PhantomData<T>,
}

pub struct RequestGen<
    Payload: Encode,
> {
    pub request_id: u32,
    pub worker: u16,
    pub payload: Payload,
}

impl<
    T: Owned,
    Payload: Encode + Compatible<T>
> Compatible<Request<T>> for RequestGen<Payload> { }
impl<
    T: Owned,
    Payload: Encode + Compatible<T>
> Compatible<RequestGen<Payload>> for Request<T> { }

impl<
    Payload: Encode,
> BaseLen for RequestGen<Payload> {
    const BASE_LEN: usize = 6 + Payload::BASE_LEN;
}

impl<
    Payload: Encode,
> Encode for RequestGen<Payload> {
    fn scratch_len(&self) -> usize {
        self.request_id.scratch_len() + self.worker.scratch_len() + self.payload.scratch_len()
    }

    fn encode(&self, cursor: &mut EncodeCursor) {
        self.request_id.encode(cursor);
        self.worker.encode(cursor);
        self.payload.encode(cursor);
    }
}

impl<T: Owned> Owned for Request<T> {
    type Lazy<'a> = RequestLazy<'a, T>;

    fn lazy_to_owned(lazy: Self::Lazy<'_>) -> DecodeResult<Self> {
        TryFrom::try_from(lazy)
    }
}

impl<'a, T: Owned> Lazy<'a> for RequestLazy<'a, T> {
    type Owned = Request<T>;
}

impl<'a, T: Owned> Compatible<RequestLazy<'a, T>> for Request<T> { }
impl<'a, T: Owned> Compatible<Request<T>> for RequestLazy<'a, T> { }

impl<'a, T: Owned> RequestLazy<'a, T> {

    pub fn request_id(&self) -> DecodeResult<u32> {
        Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 0))
    }

    pub fn worker(&self) -> DecodeResult<u16> {
        Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 4))
    }

    pub fn payload(&self) -> DecodeResult<T::Lazy<'a>> {
        Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 6))
    }
}

impl<T: BaseLen> BaseLen for Request<T> {
    const BASE_LEN: usize = 6 + T::BASE_LEN;
}

impl<T: Encode> Encode for Request<T> {
    fn scratch_len(&self) -> usize {
        self.request_id.scratch_len() + self.worker.scratch_len() + self.payload.scratch_len()
    }

    fn encode(&self, cursor: &mut EncodeCursor) {
        self.request_id.encode(cursor);
        self.worker.encode(cursor);
        self.payload.encode(cursor);
    }
}

impl<'a, T: Decode<'a>> Decode<'a> for Request<T> {
    fn decode(cursor: &DecodeCursor<'a>) -> DecodeResult<Self> {
        let request_id = Decode::decode(cursor)?;
        let worker = Decode::decode(cursor)?;
        let payload = Decode::decode(cursor)?;

        Ok(Request {
            request_id,
            worker,
            payload,
        })
    }
}

impl<'a, T: Owned> BaseLen for RequestLazy<'a, T> {
    const BASE_LEN: usize = 6 + T::BASE_LEN;
}

impl<'a, T: Owned> Encode for RequestLazy<'a, T> {
    fn scratch_len(&self) -> usize {
        let request_id: u32 = Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 0)).unwrap();
        let worker: u16 = Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 4)).unwrap();
        let payload: T::Lazy<'a> = Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 6)).unwrap();
        request_id.scratch_len() + worker.scratch_len() + payload.scratch_len()
    }

    fn encode(&self, cursor: &mut EncodeCursor) {
        let request_id: u32 = Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 0)).unwrap();
        let worker: u16 = Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 4)).unwrap();
        let payload: T::Lazy<'a> = Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 6)).unwrap();
        request_id.encode(cursor);
        worker.encode(cursor);
        payload.encode(cursor);
    }
}

impl<'a, T: Owned> Decode<'a> for RequestLazy<'a, T> {
    fn decode(cursor: &DecodeCursor<'a>) -> DecodeResult<Self> {
        let offset = cursor.offset();
        cursor.advance(Self::BASE_LEN);
        Ok(RequestLazy {
            buffer: cursor.buffer(),
            offset,
            _t: core::marker::PhantomData,
        })
    }
}

impl<'a, T: Owned> TryFrom<RequestLazy<'a, T>> for Request<T> {
    type Error = DecodeError;

    fn try_from(other: RequestLazy<'a, T>) -> Result<Self, Self::Error> {
        let cursor = DecodeCursor::at_offset(other.buffer, other.offset);
        Decode::decode(&cursor)
    }
}

impl<'a, T> Copy for RequestLazy<'a, T> { }

impl<'a, T> Clone for RequestLazy<'a, T> {
    fn clone(&self) -> Self {
        Self {
            buffer: self.buffer,
            offset: self.offset,
            _t: core::marker::PhantomData,
        }
    }
}

impl<'a, T> core::fmt::Debug for RequestLazy<'a, T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("RequestLazy")
            .finish()
    }
}

impl<'a, T: Owned> PartialEq for RequestLazy<'a, T> {
    fn eq(&self, other: &Self) -> bool {
        self.request_id().unwrap() == other.request_id().unwrap()
            && self.worker().unwrap() == other.worker().unwrap()&& self.payload().unwrap() == other.payload().unwrap()
    }
}

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct Response<T> {
    pub request_id: u32,
    pub requester: u64,
    pub requester_worker: u16,
    pub payload: T,
}

pub struct ResponseLazy<'a, T> {
    buffer: &'a [u8],
    offset: usize,
    _t: core::marker::PhantomData<T>,
}

pub struct ResponseGen<
    Payload: Encode,
> {
    pub request_id: u32,
    pub requester: u64,
    pub requester_worker: u16,
    pub payload: Payload,
}

impl<
    T: Owned,
    Payload: Encode + Compatible<T>
> Compatible<Response<T>> for ResponseGen<Payload> { }
impl<
    T: Owned,
    Payload: Encode + Compatible<T>
> Compatible<ResponseGen<Payload>> for Response<T> { }

impl<
    Payload: Encode,
> BaseLen for ResponseGen<Payload> {
    const BASE_LEN: usize = 14 + Payload::BASE_LEN;
}

impl<
    Payload: Encode,
> Encode for ResponseGen<Payload> {
    fn scratch_len(&self) -> usize {
        self.request_id.scratch_len() + self.requester.scratch_len() + self.requester_worker.scratch_len() + self.payload.scratch_len()
    }

    fn encode(&self, cursor: &mut EncodeCursor) {
        self.request_id.encode(cursor);
        self.requester.encode(cursor);
        self.requester_worker.encode(cursor);
        self.payload.encode(cursor);
    }
}

impl<T: Owned> Owned for Response<T> {
    type Lazy<'a> = ResponseLazy<'a, T>;

    fn lazy_to_owned(lazy: Self::Lazy<'_>) -> DecodeResult<Self> {
        TryFrom::try_from(lazy)
    }
}

impl<'a, T: Owned> Lazy<'a> for ResponseLazy<'a, T> {
    type Owned = Response<T>;
}

impl<'a, T: Owned> Compatible<ResponseLazy<'a, T>> for Response<T> { }
impl<'a, T: Owned> Compatible<Response<T>> for ResponseLazy<'a, T> { }

impl<'a, T: Owned> ResponseLazy<'a, T> {

    pub fn request_id(&self) -> DecodeResult<u32> {
        Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 0))
    }

    pub fn requester(&self) -> DecodeResult<u64> {
        Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 4))
    }

    pub fn requester_worker(&self) -> DecodeResult<u16> {
        Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 12))
    }

    pub fn payload(&self) -> DecodeResult<T::Lazy<'a>> {
        Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 14))
    }
}

impl<T: BaseLen> BaseLen for Response<T> {
    const BASE_LEN: usize = 14 + T::BASE_LEN;
}

impl<T: Encode> Encode for Response<T> {
    fn scratch_len(&self) -> usize {
        self.request_id.scratch_len() + self.requester.scratch_len() + self.requester_worker.scratch_len() + self.payload.scratch_len()
    }

    fn encode(&self, cursor: &mut EncodeCursor) {
        self.request_id.encode(cursor);
        self.requester.encode(cursor);
        self.requester_worker.encode(cursor);
        self.payload.encode(cursor);
    }
}

impl<'a, T: Decode<'a>> Decode<'a> for Response<T> {
    fn decode(cursor: &DecodeCursor<'a>) -> DecodeResult<Self> {
        let request_id = Decode::decode(cursor)?;
        let requester = Decode::decode(cursor)?;
        let requester_worker = Decode::decode(cursor)?;
        let payload = Decode::decode(cursor)?;

        Ok(Response {
            request_id,
            requester,
            requester_worker,
            payload,
        })
    }
}

impl<'a, T: Owned> BaseLen for ResponseLazy<'a, T> {
    const BASE_LEN: usize = 14 + T::BASE_LEN;
}

impl<'a, T: Owned> Encode for ResponseLazy<'a, T> {
    fn scratch_len(&self) -> usize {
        let request_id: u32 = Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 0)).unwrap();
        let requester: u64 = Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 4)).unwrap();
        let requester_worker: u16 = Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 12)).unwrap();
        let payload: T::Lazy<'a> = Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 14)).unwrap();
        request_id.scratch_len() + requester.scratch_len() + requester_worker.scratch_len() + payload.scratch_len()
    }

    fn encode(&self, cursor: &mut EncodeCursor) {
        let request_id: u32 = Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 0)).unwrap();
        let requester: u64 = Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 4)).unwrap();
        let requester_worker: u16 = Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 12)).unwrap();
        let payload: T::Lazy<'a> = Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 14)).unwrap();
        request_id.encode(cursor);
        requester.encode(cursor);
        requester_worker.encode(cursor);
        payload.encode(cursor);
    }
}

impl<'a, T: Owned> Decode<'a> for ResponseLazy<'a, T> {
    fn decode(cursor: &DecodeCursor<'a>) -> DecodeResult<Self> {
        let offset = cursor.offset();
        cursor.advance(Self::BASE_LEN);
        Ok(ResponseLazy {
            buffer: cursor.buffer(),
            offset,
            _t: core::marker::PhantomData,
        })
    }
}

impl<'a, T: Owned> TryFrom<ResponseLazy<'a, T>> for Response<T> {
    type Error = DecodeError;

    fn try_from(other: ResponseLazy<'a, T>) -> Result<Self, Self::Error> {
        let cursor = DecodeCursor::at_offset(other.buffer, other.offset);
        Decode::decode(&cursor)
    }
}

impl<'a, T> Copy for ResponseLazy<'a, T> { }

impl<'a, T> Clone for ResponseLazy<'a, T> {
    fn clone(&self) -> Self {
        Self {
            buffer: self.buffer,
            offset: self.offset,
            _t: core::marker::PhantomData,
        }
    }
}

impl<'a, T> core::fmt::Debug for ResponseLazy<'a, T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ResponseLazy")
            .finish()
    }
}

impl<'a, T: Owned> PartialEq for ResponseLazy<'a, T> {
    fn eq(&self, other: &Self) -> bool {
        self.request_id().unwrap() == other.request_id().unwrap()
            && self.requester().unwrap() == other.requester().unwrap()&& self.requester_worker().unwrap() == other.requester_worker().unwrap()&& self.payload().unwrap() == other.payload().unwrap()
    }
}

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct StreamItem<T> {
    pub seq: u64,
    pub payload: T,
}

pub struct StreamItemLazy<'a, T> {
    buffer: &'a [u8],
    offset: usize,
    _t: core::marker::PhantomData<T>,
}

pub struct StreamItemGen<
    Payload: Encode,
> {
    pub seq: u64,
    pub payload: Payload,
}

impl<
    T: Owned,
    Payload: Encode + Compatible<T>
> Compatible<StreamItem<T>> for StreamItemGen<Payload> { }
impl<
    T: Owned,
    Payload: Encode + Compatible<T>
> Compatible<StreamItemGen<Payload>> for StreamItem<T> { }

impl<
    Payload: Encode,
> BaseLen for StreamItemGen<Payload> {
    const BASE_LEN: usize = 8 + Payload::BASE_LEN;
}

impl<
    Payload: Encode,
> Encode for StreamItemGen<Payload> {
    fn scratch_len(&self) -> usize {
        self.seq.scratch_len() + self.payload.scratch_len()
    }

    fn encode(&self, cursor: &mut EncodeCursor) {
        self.seq.encode(cursor);
        self.payload.encode(cursor);
    }
}

impl<T: Owned> Owned for StreamItem<T> {
    type Lazy<'a> = StreamItemLazy<'a, T>;

    fn lazy_to_owned(lazy: Self::Lazy<'_>) -> DecodeResult<Self> {
        TryFrom::try_from(lazy)
    }
}

impl<'a, T: Owned> Lazy<'a> for StreamItemLazy<'a, T> {
    type Owned = StreamItem<T>;
}

impl<'a, T: Owned> Compatible<StreamItemLazy<'a, T>> for StreamItem<T> { }
impl<'a, T: Owned> Compatible<StreamItem<T>> for StreamItemLazy<'a, T> { }

impl<'a, T: Owned> StreamItemLazy<'a, T> {

    pub fn seq(&self) -> DecodeResult<u64> {
        Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 0))
    }

    pub fn payload(&self) -> DecodeResult<T::Lazy<'a>> {
        Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 8))
    }
}

impl<T: BaseLen> BaseLen for StreamItem<T> {
    const BASE_LEN: usize = 8 + T::BASE_LEN;
}

impl<T: Encode> Encode for StreamItem<T> {
    fn scratch_len(&self) -> usize {
        self.seq.scratch_len() + self.payload.scratch_len()
    }

    fn encode(&self, cursor: &mut EncodeCursor) {
        self.seq.encode(cursor);
        self.payload.encode(cursor);
    }
}

impl<'a, T: Decode<'a>> Decode<'a> for StreamItem<T> {
    fn decode(cursor: &DecodeCursor<'a>) -> DecodeResult<Self> {
        let seq = Decode::decode(cursor)?;
        let payload = Decode::decode(cursor)?;

        Ok(StreamItem {
            seq,
            payload,
        })
    }
}

impl<'a, T: Owned> BaseLen for StreamItemLazy<'a, T> {
    const BASE_LEN: usize = 8 + T::BASE_LEN;
}

impl<'a, T: Owned> Encode for StreamItemLazy<'a, T> {
    fn scratch_len(&self) -> usize {
        let seq: u64 = Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 0)).unwrap();
        let payload: T::Lazy<'a> = Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 8)).unwrap();
        seq.scratch_len() + payload.scratch_len()
    }

    fn encode(&self, cursor: &mut EncodeCursor) {
        let seq: u64 = Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 0)).unwrap();
        let payload: T::Lazy<'a> = Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 8)).unwrap();
        seq.encode(cursor);
        payload.encode(cursor);
    }
}

impl<'a, T: Owned> Decode<'a> for StreamItemLazy<'a, T> {
    fn decode(cursor: &DecodeCursor<'a>) -> DecodeResult<Self> {
        let offset = cursor.offset();
        cursor.advance(Self::BASE_LEN);
        Ok(StreamItemLazy {
            buffer: cursor.buffer(),
            offset,
            _t: core::marker::PhantomData,
        })
    }
}

impl<'a, T: Owned> TryFrom<StreamItemLazy<'a, T>> for StreamItem<T> {
    type Error = DecodeError;

    fn try_from(other: StreamItemLazy<'a, T>) -> Result<Self, Self::Error> {
        let cursor = DecodeCursor::at_offset(other.buffer, other.offset);
        Decode::decode(&cursor)
    }
}

impl<'a, T> Copy for StreamItemLazy<'a, T> { }

impl<'a, T> Clone for StreamItemLazy<'a, T> {
    fn clone(&self) -> Self {
        Self {
            buffer: self.buffer,
            offset: self.offset,
            _t: core::marker::PhantomData,
        }
    }
}

impl<'a, T> core::fmt::Debug for StreamItemLazy<'a, T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("StreamItemLazy")
            .finish()
    }
}

impl<'a, T: Owned> PartialEq for StreamItemLazy<'a, T> {
    fn eq(&self, other: &Self) -> bool {
        self.seq().unwrap() == other.seq().unwrap()
            && self.payload().unwrap() == other.payload().unwrap()
    }
}

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct MultiStreamId {
    pub owner: u64,
    pub id: u32,
}

pub struct MultiStreamIdLazy<'a> {
    buffer: &'a [u8],
    offset: usize,
}

pub struct MultiStreamIdGen<> {
    pub owner: u64,
    pub id: u32,
}

impl<> Compatible<MultiStreamId> for MultiStreamIdGen<> { }
impl<> Compatible<MultiStreamIdGen<>> for MultiStreamId { }

impl<> BaseLen for MultiStreamIdGen<> {
    const BASE_LEN: usize = 12;
}

impl<> Encode for MultiStreamIdGen<> {
    fn scratch_len(&self) -> usize {
        self.owner.scratch_len() + self.id.scratch_len()
    }

    fn encode(&self, cursor: &mut EncodeCursor) {
        self.owner.encode(cursor);
        self.id.encode(cursor);
    }
}

impl Owned for MultiStreamId {
    type Lazy<'a> = MultiStreamIdLazy<'a>;

    fn lazy_to_owned(lazy: Self::Lazy<'_>) -> DecodeResult<Self> {
        TryFrom::try_from(lazy)
    }
}

impl<'a> Lazy<'a> for MultiStreamIdLazy<'a> {
    type Owned = MultiStreamId;
}

impl<'a> Compatible<MultiStreamIdLazy<'a>> for MultiStreamId { }
impl<'a> Compatible<MultiStreamId> for MultiStreamIdLazy<'a> { }

impl<'a> MultiStreamIdLazy<'a> {

    pub fn owner(&self) -> DecodeResult<u64> {
        Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 0))
    }

    pub fn id(&self) -> DecodeResult<u32> {
        Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 8))
    }
}

impl BaseLen for MultiStreamId {
    const BASE_LEN: usize = 12;
}

impl Encode for MultiStreamId {
    fn scratch_len(&self) -> usize {
        self.owner.scratch_len() + self.id.scratch_len()
    }

    fn encode(&self, cursor: &mut EncodeCursor) {
        self.owner.encode(cursor);
        self.id.encode(cursor);
    }
}

impl<'a> Decode<'a> for MultiStreamId {
    fn decode(cursor: &DecodeCursor<'a>) -> DecodeResult<Self> {
        let owner = Decode::decode(cursor)?;
        let id = Decode::decode(cursor)?;

        Ok(MultiStreamId {
            owner,
            id,
        })
    }
}

impl<'a> BaseLen for MultiStreamIdLazy<'a> {
    const BASE_LEN: usize = 12;
}

impl<'a> Encode for MultiStreamIdLazy<'a> {
    fn scratch_len(&self) -> usize {
        let owner: u64 = Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 0)).unwrap();
        let id: u32 = Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 8)).unwrap();
        owner.scratch_len() + id.scratch_len()
    }

    fn encode(&self, cursor: &mut EncodeCursor) {
        let owner: u64 = Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 0)).unwrap();
        let id: u32 = Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 8)).unwrap();
        owner.encode(cursor);
        id.encode(cursor);
    }
}

impl<'a> Decode<'a> for MultiStreamIdLazy<'a> {
    fn decode(cursor: &DecodeCursor<'a>) -> DecodeResult<Self> {
        let offset = cursor.offset();
        cursor.advance(Self::BASE_LEN);
        Ok(MultiStreamIdLazy {
            buffer: cursor.buffer(),
            offset,
        })
    }
}

impl<'a> TryFrom<MultiStreamIdLazy<'a>> for MultiStreamId {
    type Error = DecodeError;

    fn try_from(other: MultiStreamIdLazy<'a>) -> Result<Self, Self::Error> {
        let cursor = DecodeCursor::at_offset(other.buffer, other.offset);
        Decode::decode(&cursor)
    }
}

impl<'a> Copy for MultiStreamIdLazy<'a> { }

impl<'a> Clone for MultiStreamIdLazy<'a> {
    fn clone(&self) -> Self {
        Self {
            buffer: self.buffer,
            offset: self.offset,
        }
    }
}

impl<'a> core::fmt::Debug for MultiStreamIdLazy<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("MultiStreamIdLazy")
            .finish()
    }
}

impl<'a> PartialEq for MultiStreamIdLazy<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.owner().unwrap() == other.owner().unwrap()
            && self.id().unwrap() == other.id().unwrap()
    }
}

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct MultiStreamItem<T> {
    pub stream_id: MultiStreamId,
    pub seq: u64,
    pub payload: Option<T>,
}

pub struct MultiStreamItemLazy<'a, T> {
    buffer: &'a [u8],
    offset: usize,
    _t: core::marker::PhantomData<T>,
}

pub struct MultiStreamItemGen<
    StreamId: Encode + Compatible<MultiStreamId>,
    Payload: Encode,
> {
    pub stream_id: StreamId,
    pub seq: u64,
    pub payload: Payload,
}

impl<
    T: Owned,
    StreamId: Encode + Compatible<MultiStreamId>,
    Payload: Encode + Compatible<Option<T>>
> Compatible<MultiStreamItem<T>> for MultiStreamItemGen<StreamId, Payload> { }
impl<
    T: Owned,
    StreamId: Encode + Compatible<MultiStreamId>,
    Payload: Encode + Compatible<Option<T>>
> Compatible<MultiStreamItemGen<StreamId, Payload>> for MultiStreamItem<T> { }

impl<
    StreamId: Encode + Compatible<MultiStreamId>,
    Payload: Encode,
> BaseLen for MultiStreamItemGen<StreamId, Payload> {
    const BASE_LEN: usize = 8 + StreamId::BASE_LEN + Payload::BASE_LEN;
}

impl<
    StreamId: Encode + Compatible<MultiStreamId>,
    Payload: Encode,
> Encode for MultiStreamItemGen<StreamId, Payload> {
    fn scratch_len(&self) -> usize {
        self.stream_id.scratch_len() + self.seq.scratch_len() + self.payload.scratch_len()
    }

    fn encode(&self, cursor: &mut EncodeCursor) {
        self.stream_id.encode(cursor);
        self.seq.encode(cursor);
        self.payload.encode(cursor);
    }
}

impl<T: Owned> Owned for MultiStreamItem<T> {
    type Lazy<'a> = MultiStreamItemLazy<'a, T>;

    fn lazy_to_owned(lazy: Self::Lazy<'_>) -> DecodeResult<Self> {
        TryFrom::try_from(lazy)
    }
}

impl<'a, T: Owned> Lazy<'a> for MultiStreamItemLazy<'a, T> {
    type Owned = MultiStreamItem<T>;
}

impl<'a, T: Owned> Compatible<MultiStreamItemLazy<'a, T>> for MultiStreamItem<T> { }
impl<'a, T: Owned> Compatible<MultiStreamItem<T>> for MultiStreamItemLazy<'a, T> { }

impl<'a, T: Owned> MultiStreamItemLazy<'a, T> {

    pub fn stream_id(&self) -> DecodeResult<MultiStreamIdLazy<'a>> {
        Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 0))
    }

    pub fn seq(&self) -> DecodeResult<u64> {
        Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 12))
    }

    pub fn payload(&self) -> DecodeResult<Option<T::Lazy<'a>>> {
        Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 20))
    }
}

impl<T: BaseLen> BaseLen for MultiStreamItem<T> {
    const BASE_LEN: usize = 21 + T::BASE_LEN;
}

impl<T: Encode> Encode for MultiStreamItem<T> {
    fn scratch_len(&self) -> usize {
        self.stream_id.scratch_len() + self.seq.scratch_len() + self.payload.scratch_len()
    }

    fn encode(&self, cursor: &mut EncodeCursor) {
        self.stream_id.encode(cursor);
        self.seq.encode(cursor);
        self.payload.encode(cursor);
    }
}

impl<'a, T: Decode<'a>> Decode<'a> for MultiStreamItem<T> {
    fn decode(cursor: &DecodeCursor<'a>) -> DecodeResult<Self> {
        let stream_id = Decode::decode(cursor)?;
        let seq = Decode::decode(cursor)?;
        let payload = Decode::decode(cursor)?;

        Ok(MultiStreamItem {
            stream_id,
            seq,
            payload,
        })
    }
}

impl<'a, T: Owned> BaseLen for MultiStreamItemLazy<'a, T> {
    const BASE_LEN: usize = 21 + T::BASE_LEN;
}

impl<'a, T: Owned> Encode for MultiStreamItemLazy<'a, T> {
    fn scratch_len(&self) -> usize {
        let stream_id: MultiStreamIdLazy<'a> = Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 0)).unwrap();
        let seq: u64 = Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 12)).unwrap();
        let payload: Option<T::Lazy<'a>> = Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 20)).unwrap();
        stream_id.scratch_len() + seq.scratch_len() + payload.scratch_len()
    }

    fn encode(&self, cursor: &mut EncodeCursor) {
        let stream_id: MultiStreamIdLazy<'a> = Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 0)).unwrap();
        let seq: u64 = Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 12)).unwrap();
        let payload: Option<T::Lazy<'a>> = Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 20)).unwrap();
        stream_id.encode(cursor);
        seq.encode(cursor);
        payload.encode(cursor);
    }
}

impl<'a, T: Owned> Decode<'a> for MultiStreamItemLazy<'a, T> {
    fn decode(cursor: &DecodeCursor<'a>) -> DecodeResult<Self> {
        let offset = cursor.offset();
        cursor.advance(Self::BASE_LEN);
        Ok(MultiStreamItemLazy {
            buffer: cursor.buffer(),
            offset,
            _t: core::marker::PhantomData,
        })
    }
}

impl<'a, T: Owned> TryFrom<MultiStreamItemLazy<'a, T>> for MultiStreamItem<T> {
    type Error = DecodeError;

    fn try_from(other: MultiStreamItemLazy<'a, T>) -> Result<Self, Self::Error> {
        let cursor = DecodeCursor::at_offset(other.buffer, other.offset);
        Decode::decode(&cursor)
    }
}

impl<'a, T> Copy for MultiStreamItemLazy<'a, T> { }

impl<'a, T> Clone for MultiStreamItemLazy<'a, T> {
    fn clone(&self) -> Self {
        Self {
            buffer: self.buffer,
            offset: self.offset,
            _t: core::marker::PhantomData,
        }
    }
}

impl<'a, T> core::fmt::Debug for MultiStreamItemLazy<'a, T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("MultiStreamItemLazy")
            .finish()
    }
}

impl<'a, T: Owned> PartialEq for MultiStreamItemLazy<'a, T> {
    fn eq(&self, other: &Self) -> bool {
        self.stream_id().unwrap() == other.stream_id().unwrap()
            && self.seq().unwrap() == other.seq().unwrap()&& self.payload().unwrap() == other.payload().unwrap()
    }
}

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct PropertyInitState<T> {
    pub value: T,
}

pub struct PropertyInitStateLazy<'a, T> {
    buffer: &'a [u8],
    offset: usize,
    _t: core::marker::PhantomData<T>,
}

pub struct PropertyInitStateGen<
    Value: Encode,
> {
    pub value: Value,
}

impl<
    T: Owned,
    Value: Encode + Compatible<T>
> Compatible<PropertyInitState<T>> for PropertyInitStateGen<Value> { }
impl<
    T: Owned,
    Value: Encode + Compatible<T>
> Compatible<PropertyInitStateGen<Value>> for PropertyInitState<T> { }

impl<
    Value: Encode,
> BaseLen for PropertyInitStateGen<Value> {
    const BASE_LEN: usize = Value::BASE_LEN;
}

impl<
    Value: Encode,
> Encode for PropertyInitStateGen<Value> {
    fn scratch_len(&self) -> usize {
        self.value.scratch_len()
    }

    fn encode(&self, cursor: &mut EncodeCursor) {
        self.value.encode(cursor);
    }
}

impl<T: Owned> Owned for PropertyInitState<T> {
    type Lazy<'a> = PropertyInitStateLazy<'a, T>;

    fn lazy_to_owned(lazy: Self::Lazy<'_>) -> DecodeResult<Self> {
        TryFrom::try_from(lazy)
    }
}

impl<'a, T: Owned> Lazy<'a> for PropertyInitStateLazy<'a, T> {
    type Owned = PropertyInitState<T>;
}

impl<'a, T: Owned> Compatible<PropertyInitStateLazy<'a, T>> for PropertyInitState<T> { }
impl<'a, T: Owned> Compatible<PropertyInitState<T>> for PropertyInitStateLazy<'a, T> { }

impl<'a, T: Owned> PropertyInitStateLazy<'a, T> {

    pub fn value(&self) -> DecodeResult<T::Lazy<'a>> {
        Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 0))
    }
}

impl<T: BaseLen> BaseLen for PropertyInitState<T> {
    const BASE_LEN: usize = T::BASE_LEN;
}

impl<T: Encode> Encode for PropertyInitState<T> {
    fn scratch_len(&self) -> usize {
        self.value.scratch_len()
    }

    fn encode(&self, cursor: &mut EncodeCursor) {
        self.value.encode(cursor);
    }
}

impl<'a, T: Decode<'a>> Decode<'a> for PropertyInitState<T> {
    fn decode(cursor: &DecodeCursor<'a>) -> DecodeResult<Self> {
        let value = Decode::decode(cursor)?;

        Ok(PropertyInitState {
            value,
        })
    }
}

impl<'a, T: Owned> BaseLen for PropertyInitStateLazy<'a, T> {
    const BASE_LEN: usize = T::BASE_LEN;
}

impl<'a, T: Owned> Encode for PropertyInitStateLazy<'a, T> {
    fn scratch_len(&self) -> usize {
        let value: T::Lazy<'a> = Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 0)).unwrap();
        value.scratch_len()
    }

    fn encode(&self, cursor: &mut EncodeCursor) {
        let value: T::Lazy<'a> = Decode::decode(&DecodeCursor::at_offset(self.buffer, self.offset + 0)).unwrap();
        value.encode(cursor);
    }
}

impl<'a, T: Owned> Decode<'a> for PropertyInitStateLazy<'a, T> {
    fn decode(cursor: &DecodeCursor<'a>) -> DecodeResult<Self> {
        let offset = cursor.offset();
        cursor.advance(Self::BASE_LEN);
        Ok(PropertyInitStateLazy {
            buffer: cursor.buffer(),
            offset,
            _t: core::marker::PhantomData,
        })
    }
}

impl<'a, T: Owned> TryFrom<PropertyInitStateLazy<'a, T>> for PropertyInitState<T> {
    type Error = DecodeError;

    fn try_from(other: PropertyInitStateLazy<'a, T>) -> Result<Self, Self::Error> {
        let cursor = DecodeCursor::at_offset(other.buffer, other.offset);
        Decode::decode(&cursor)
    }
}

impl<'a, T> Copy for PropertyInitStateLazy<'a, T> { }

impl<'a, T> Clone for PropertyInitStateLazy<'a, T> {
    fn clone(&self) -> Self {
        Self {
            buffer: self.buffer,
            offset: self.offset,
            _t: core::marker::PhantomData,
        }
    }
}

impl<'a, T> core::fmt::Debug for PropertyInitStateLazy<'a, T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("PropertyInitStateLazy")
            .finish()
    }
}

impl<'a, T: Owned> PartialEq for PropertyInitStateLazy<'a, T> {
    fn eq(&self, other: &Self) -> bool {
        self.value().unwrap() == other.value().unwrap()
    }
}

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Default)]
pub struct PropertyObserverConfig {}

pub struct PropertyObserverConfigLazy<'a> {
    buffer: &'a [u8],
    offset: usize,
}

pub struct PropertyObserverConfigGen<> {}

impl<> Compatible<PropertyObserverConfig> for PropertyObserverConfigGen<> { }
impl<> Compatible<PropertyObserverConfigGen<>> for PropertyObserverConfig { }

impl<> BaseLen for PropertyObserverConfigGen<> {
    const BASE_LEN: usize = 0;
}

impl<> Encode for PropertyObserverConfigGen<> {
    fn scratch_len(&self) -> usize {
        0
    }

    fn encode(&self, _: &mut EncodeCursor) {}
}

impl Owned for PropertyObserverConfig {
    type Lazy<'a> = PropertyObserverConfigLazy<'a>;

    fn lazy_to_owned(lazy: Self::Lazy<'_>) -> DecodeResult<Self> {
        TryFrom::try_from(lazy)
    }
}

impl<'a> Lazy<'a> for PropertyObserverConfigLazy<'a> {
    type Owned = PropertyObserverConfig;
}

impl<'a> Compatible<PropertyObserverConfigLazy<'a>> for PropertyObserverConfig { }
impl<'a> Compatible<PropertyObserverConfig> for PropertyObserverConfigLazy<'a> { }

impl<'a> PropertyObserverConfigLazy<'a> {}

impl BaseLen for PropertyObserverConfig {
    const BASE_LEN: usize = 0;
}

impl Encode for PropertyObserverConfig {
    fn scratch_len(&self) -> usize {
        0
    }

    fn encode(&self, _: &mut EncodeCursor) {}
}

impl<'a> Decode<'a> for PropertyObserverConfig {
    fn decode(_: &DecodeCursor<'a>) -> DecodeResult<Self> {

        Ok(PropertyObserverConfig {})
    }
}

impl<'a> BaseLen for PropertyObserverConfigLazy<'a> {
    const BASE_LEN: usize = 0;
}

impl<'a> Encode for PropertyObserverConfigLazy<'a> {
    fn scratch_len(&self) -> usize {
        0
    }

    fn encode(&self, _: &mut EncodeCursor) {}
}

impl<'a> Decode<'a> for PropertyObserverConfigLazy<'a> {
    fn decode(cursor: &DecodeCursor<'a>) -> DecodeResult<Self> {
        let offset = cursor.offset();
        cursor.advance(Self::BASE_LEN);
        Ok(PropertyObserverConfigLazy {
            buffer: cursor.buffer(),
            offset,
        })
    }
}

impl<'a> TryFrom<PropertyObserverConfigLazy<'a>> for PropertyObserverConfig {
    type Error = DecodeError;

    fn try_from(other: PropertyObserverConfigLazy<'a>) -> Result<Self, Self::Error> {
        let cursor = DecodeCursor::at_offset(other.buffer, other.offset);
        Decode::decode(&cursor)
    }
}

impl<'a> Copy for PropertyObserverConfigLazy<'a> { }

impl<'a> Clone for PropertyObserverConfigLazy<'a> {
    fn clone(&self) -> Self {
        Self {
            buffer: self.buffer,
            offset: self.offset,
        }
    }
}

impl<'a> core::fmt::Debug for PropertyObserverConfigLazy<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("PropertyObserverConfigLazy")
            .finish()
    }
}

impl<'a> PartialEq for PropertyObserverConfigLazy<'a> {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Default)]
pub struct PropertyOwnerConfig {}

pub struct PropertyOwnerConfigLazy<'a> {
    buffer: &'a [u8],
    offset: usize,
}

pub struct PropertyOwnerConfigGen<> {}

impl<> Compatible<PropertyOwnerConfig> for PropertyOwnerConfigGen<> { }
impl<> Compatible<PropertyOwnerConfigGen<>> for PropertyOwnerConfig { }

impl<> BaseLen for PropertyOwnerConfigGen<> {
    const BASE_LEN: usize = 0;
}

impl<> Encode for PropertyOwnerConfigGen<> {
    fn scratch_len(&self) -> usize {
        0
    }

    fn encode(&self, _: &mut EncodeCursor) {}
}

impl Owned for PropertyOwnerConfig {
    type Lazy<'a> = PropertyOwnerConfigLazy<'a>;

    fn lazy_to_owned(lazy: Self::Lazy<'_>) -> DecodeResult<Self> {
        TryFrom::try_from(lazy)
    }
}

impl<'a> Lazy<'a> for PropertyOwnerConfigLazy<'a> {
    type Owned = PropertyOwnerConfig;
}

impl<'a> Compatible<PropertyOwnerConfigLazy<'a>> for PropertyOwnerConfig { }
impl<'a> Compatible<PropertyOwnerConfig> for PropertyOwnerConfigLazy<'a> { }

impl<'a> PropertyOwnerConfigLazy<'a> {}

impl BaseLen for PropertyOwnerConfig {
    const BASE_LEN: usize = 0;
}

impl Encode for PropertyOwnerConfig {
    fn scratch_len(&self) -> usize {
        0
    }

    fn encode(&self, _: &mut EncodeCursor) {}
}

impl<'a> Decode<'a> for PropertyOwnerConfig {
    fn decode(_: &DecodeCursor<'a>) -> DecodeResult<Self> {

        Ok(PropertyOwnerConfig {})
    }
}

impl<'a> BaseLen for PropertyOwnerConfigLazy<'a> {
    const BASE_LEN: usize = 0;
}

impl<'a> Encode for PropertyOwnerConfigLazy<'a> {
    fn scratch_len(&self) -> usize {
        0
    }

    fn encode(&self, _: &mut EncodeCursor) {}
}

impl<'a> Decode<'a> for PropertyOwnerConfigLazy<'a> {
    fn decode(cursor: &DecodeCursor<'a>) -> DecodeResult<Self> {
        let offset = cursor.offset();
        cursor.advance(Self::BASE_LEN);
        Ok(PropertyOwnerConfigLazy {
            buffer: cursor.buffer(),
            offset,
        })
    }
}

impl<'a> TryFrom<PropertyOwnerConfigLazy<'a>> for PropertyOwnerConfig {
    type Error = DecodeError;

    fn try_from(other: PropertyOwnerConfigLazy<'a>) -> Result<Self, Self::Error> {
        let cursor = DecodeCursor::at_offset(other.buffer, other.offset);
        Decode::decode(&cursor)
    }
}

impl<'a> Copy for PropertyOwnerConfigLazy<'a> { }

impl<'a> Clone for PropertyOwnerConfigLazy<'a> {
    fn clone(&self) -> Self {
        Self {
            buffer: self.buffer,
            offset: self.offset,
        }
    }
}

impl<'a> core::fmt::Debug for PropertyOwnerConfigLazy<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("PropertyOwnerConfigLazy")
            .finish()
    }
}

impl<'a> PartialEq for PropertyOwnerConfigLazy<'a> {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Default)]
pub struct RequestInitState {}

pub struct RequestInitStateLazy<'a> {
    buffer: &'a [u8],
    offset: usize,
}

pub struct RequestInitStateGen<> {}

impl<> Compatible<RequestInitState> for RequestInitStateGen<> { }
impl<> Compatible<RequestInitStateGen<>> for RequestInitState { }

impl<> BaseLen for RequestInitStateGen<> {
    const BASE_LEN: usize = 0;
}

impl<> Encode for RequestInitStateGen<> {
    fn scratch_len(&self) -> usize {
        0
    }

    fn encode(&self, _: &mut EncodeCursor) {}
}

impl Owned for RequestInitState {
    type Lazy<'a> = RequestInitStateLazy<'a>;

    fn lazy_to_owned(lazy: Self::Lazy<'_>) -> DecodeResult<Self> {
        TryFrom::try_from(lazy)
    }
}

impl<'a> Lazy<'a> for RequestInitStateLazy<'a> {
    type Owned = RequestInitState;
}

impl<'a> Compatible<RequestInitStateLazy<'a>> for RequestInitState { }
impl<'a> Compatible<RequestInitState> for RequestInitStateLazy<'a> { }

impl<'a> RequestInitStateLazy<'a> {}

impl BaseLen for RequestInitState {
    const BASE_LEN: usize = 0;
}

impl Encode for RequestInitState {
    fn scratch_len(&self) -> usize {
        0
    }

    fn encode(&self, _: &mut EncodeCursor) {}
}

impl<'a> Decode<'a> for RequestInitState {
    fn decode(_: &DecodeCursor<'a>) -> DecodeResult<Self> {

        Ok(RequestInitState {})
    }
}

impl<'a> BaseLen for RequestInitStateLazy<'a> {
    const BASE_LEN: usize = 0;
}

impl<'a> Encode for RequestInitStateLazy<'a> {
    fn scratch_len(&self) -> usize {
        0
    }

    fn encode(&self, _: &mut EncodeCursor) {}
}

impl<'a> Decode<'a> for RequestInitStateLazy<'a> {
    fn decode(cursor: &DecodeCursor<'a>) -> DecodeResult<Self> {
        let offset = cursor.offset();
        cursor.advance(Self::BASE_LEN);
        Ok(RequestInitStateLazy {
            buffer: cursor.buffer(),
            offset,
        })
    }
}

impl<'a> TryFrom<RequestInitStateLazy<'a>> for RequestInitState {
    type Error = DecodeError;

    fn try_from(other: RequestInitStateLazy<'a>) -> Result<Self, Self::Error> {
        let cursor = DecodeCursor::at_offset(other.buffer, other.offset);
        Decode::decode(&cursor)
    }
}

impl<'a> Copy for RequestInitStateLazy<'a> { }

impl<'a> Clone for RequestInitStateLazy<'a> {
    fn clone(&self) -> Self {
        Self {
            buffer: self.buffer,
            offset: self.offset,
        }
    }
}

impl<'a> core::fmt::Debug for RequestInitStateLazy<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("RequestInitStateLazy")
            .finish()
    }
}

impl<'a> PartialEq for RequestInitStateLazy<'a> {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Default)]
pub struct RequestClientConfig {}

pub struct RequestClientConfigLazy<'a> {
    buffer: &'a [u8],
    offset: usize,
}

pub struct RequestClientConfigGen<> {}

impl<> Compatible<RequestClientConfig> for RequestClientConfigGen<> { }
impl<> Compatible<RequestClientConfigGen<>> for RequestClientConfig { }

impl<> BaseLen for RequestClientConfigGen<> {
    const BASE_LEN: usize = 0;
}

impl<> Encode for RequestClientConfigGen<> {
    fn scratch_len(&self) -> usize {
        0
    }

    fn encode(&self, _: &mut EncodeCursor) {}
}

impl Owned for RequestClientConfig {
    type Lazy<'a> = RequestClientConfigLazy<'a>;

    fn lazy_to_owned(lazy: Self::Lazy<'_>) -> DecodeResult<Self> {
        TryFrom::try_from(lazy)
    }
}

impl<'a> Lazy<'a> for RequestClientConfigLazy<'a> {
    type Owned = RequestClientConfig;
}

impl<'a> Compatible<RequestClientConfigLazy<'a>> for RequestClientConfig { }
impl<'a> Compatible<RequestClientConfig> for RequestClientConfigLazy<'a> { }

impl<'a> RequestClientConfigLazy<'a> {}

impl BaseLen for RequestClientConfig {
    const BASE_LEN: usize = 0;
}

impl Encode for RequestClientConfig {
    fn scratch_len(&self) -> usize {
        0
    }

    fn encode(&self, _: &mut EncodeCursor) {}
}

impl<'a> Decode<'a> for RequestClientConfig {
    fn decode(_: &DecodeCursor<'a>) -> DecodeResult<Self> {

        Ok(RequestClientConfig {})
    }
}

impl<'a> BaseLen for RequestClientConfigLazy<'a> {
    const BASE_LEN: usize = 0;
}

impl<'a> Encode for RequestClientConfigLazy<'a> {
    fn scratch_len(&self) -> usize {
        0
    }

    fn encode(&self, _: &mut EncodeCursor) {}
}

impl<'a> Decode<'a> for RequestClientConfigLazy<'a> {
    fn decode(cursor: &DecodeCursor<'a>) -> DecodeResult<Self> {
        let offset = cursor.offset();
        cursor.advance(Self::BASE_LEN);
        Ok(RequestClientConfigLazy {
            buffer: cursor.buffer(),
            offset,
        })
    }
}

impl<'a> TryFrom<RequestClientConfigLazy<'a>> for RequestClientConfig {
    type Error = DecodeError;

    fn try_from(other: RequestClientConfigLazy<'a>) -> Result<Self, Self::Error> {
        let cursor = DecodeCursor::at_offset(other.buffer, other.offset);
        Decode::decode(&cursor)
    }
}

impl<'a> Copy for RequestClientConfigLazy<'a> { }

impl<'a> Clone for RequestClientConfigLazy<'a> {
    fn clone(&self) -> Self {
        Self {
            buffer: self.buffer,
            offset: self.offset,
        }
    }
}

impl<'a> core::fmt::Debug for RequestClientConfigLazy<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("RequestClientConfigLazy")
            .finish()
    }
}

impl<'a> PartialEq for RequestClientConfigLazy<'a> {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Default)]
pub struct RequestServerConfig {}

pub struct RequestServerConfigLazy<'a> {
    buffer: &'a [u8],
    offset: usize,
}

pub struct RequestServerConfigGen<> {}

impl<> Compatible<RequestServerConfig> for RequestServerConfigGen<> { }
impl<> Compatible<RequestServerConfigGen<>> for RequestServerConfig { }

impl<> BaseLen for RequestServerConfigGen<> {
    const BASE_LEN: usize = 0;
}

impl<> Encode for RequestServerConfigGen<> {
    fn scratch_len(&self) -> usize {
        0
    }

    fn encode(&self, _: &mut EncodeCursor) {}
}

impl Owned for RequestServerConfig {
    type Lazy<'a> = RequestServerConfigLazy<'a>;

    fn lazy_to_owned(lazy: Self::Lazy<'_>) -> DecodeResult<Self> {
        TryFrom::try_from(lazy)
    }
}

impl<'a> Lazy<'a> for RequestServerConfigLazy<'a> {
    type Owned = RequestServerConfig;
}

impl<'a> Compatible<RequestServerConfigLazy<'a>> for RequestServerConfig { }
impl<'a> Compatible<RequestServerConfig> for RequestServerConfigLazy<'a> { }

impl<'a> RequestServerConfigLazy<'a> {}

impl BaseLen for RequestServerConfig {
    const BASE_LEN: usize = 0;
}

impl Encode for RequestServerConfig {
    fn scratch_len(&self) -> usize {
        0
    }

    fn encode(&self, _: &mut EncodeCursor) {}
}

impl<'a> Decode<'a> for RequestServerConfig {
    fn decode(_: &DecodeCursor<'a>) -> DecodeResult<Self> {

        Ok(RequestServerConfig {})
    }
}

impl<'a> BaseLen for RequestServerConfigLazy<'a> {
    const BASE_LEN: usize = 0;
}

impl<'a> Encode for RequestServerConfigLazy<'a> {
    fn scratch_len(&self) -> usize {
        0
    }

    fn encode(&self, _: &mut EncodeCursor) {}
}

impl<'a> Decode<'a> for RequestServerConfigLazy<'a> {
    fn decode(cursor: &DecodeCursor<'a>) -> DecodeResult<Self> {
        let offset = cursor.offset();
        cursor.advance(Self::BASE_LEN);
        Ok(RequestServerConfigLazy {
            buffer: cursor.buffer(),
            offset,
        })
    }
}

impl<'a> TryFrom<RequestServerConfigLazy<'a>> for RequestServerConfig {
    type Error = DecodeError;

    fn try_from(other: RequestServerConfigLazy<'a>) -> Result<Self, Self::Error> {
        let cursor = DecodeCursor::at_offset(other.buffer, other.offset);
        Decode::decode(&cursor)
    }
}

impl<'a> Copy for RequestServerConfigLazy<'a> { }

impl<'a> Clone for RequestServerConfigLazy<'a> {
    fn clone(&self) -> Self {
        Self {
            buffer: self.buffer,
            offset: self.offset,
        }
    }
}

impl<'a> core::fmt::Debug for RequestServerConfigLazy<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("RequestServerConfigLazy")
            .finish()
    }
}

impl<'a> PartialEq for RequestServerConfigLazy<'a> {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Default)]
pub struct StreamInitState {}

pub struct StreamInitStateLazy<'a> {
    buffer: &'a [u8],
    offset: usize,
}

pub struct StreamInitStateGen<> {}

impl<> Compatible<StreamInitState> for StreamInitStateGen<> { }
impl<> Compatible<StreamInitStateGen<>> for StreamInitState { }

impl<> BaseLen for StreamInitStateGen<> {
    const BASE_LEN: usize = 0;
}

impl<> Encode for StreamInitStateGen<> {
    fn scratch_len(&self) -> usize {
        0
    }

    fn encode(&self, _: &mut EncodeCursor) {}
}

impl Owned for StreamInitState {
    type Lazy<'a> = StreamInitStateLazy<'a>;

    fn lazy_to_owned(lazy: Self::Lazy<'_>) -> DecodeResult<Self> {
        TryFrom::try_from(lazy)
    }
}

impl<'a> Lazy<'a> for StreamInitStateLazy<'a> {
    type Owned = StreamInitState;
}

impl<'a> Compatible<StreamInitStateLazy<'a>> for StreamInitState { }
impl<'a> Compatible<StreamInitState> for StreamInitStateLazy<'a> { }

impl<'a> StreamInitStateLazy<'a> {}

impl BaseLen for StreamInitState {
    const BASE_LEN: usize = 0;
}

impl Encode for StreamInitState {
    fn scratch_len(&self) -> usize {
        0
    }

    fn encode(&self, _: &mut EncodeCursor) {}
}

impl<'a> Decode<'a> for StreamInitState {
    fn decode(_: &DecodeCursor<'a>) -> DecodeResult<Self> {

        Ok(StreamInitState {})
    }
}

impl<'a> BaseLen for StreamInitStateLazy<'a> {
    const BASE_LEN: usize = 0;
}

impl<'a> Encode for StreamInitStateLazy<'a> {
    fn scratch_len(&self) -> usize {
        0
    }

    fn encode(&self, _: &mut EncodeCursor) {}
}

impl<'a> Decode<'a> for StreamInitStateLazy<'a> {
    fn decode(cursor: &DecodeCursor<'a>) -> DecodeResult<Self> {
        let offset = cursor.offset();
        cursor.advance(Self::BASE_LEN);
        Ok(StreamInitStateLazy {
            buffer: cursor.buffer(),
            offset,
        })
    }
}

impl<'a> TryFrom<StreamInitStateLazy<'a>> for StreamInitState {
    type Error = DecodeError;

    fn try_from(other: StreamInitStateLazy<'a>) -> Result<Self, Self::Error> {
        let cursor = DecodeCursor::at_offset(other.buffer, other.offset);
        Decode::decode(&cursor)
    }
}

impl<'a> Copy for StreamInitStateLazy<'a> { }

impl<'a> Clone for StreamInitStateLazy<'a> {
    fn clone(&self) -> Self {
        Self {
            buffer: self.buffer,
            offset: self.offset,
        }
    }
}

impl<'a> core::fmt::Debug for StreamInitStateLazy<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("StreamInitStateLazy")
            .finish()
    }
}

impl<'a> PartialEq for StreamInitStateLazy<'a> {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Default)]
pub struct StreamReceiverConfig {}

pub struct StreamReceiverConfigLazy<'a> {
    buffer: &'a [u8],
    offset: usize,
}

pub struct StreamReceiverConfigGen<> {}

impl<> Compatible<StreamReceiverConfig> for StreamReceiverConfigGen<> { }
impl<> Compatible<StreamReceiverConfigGen<>> for StreamReceiverConfig { }

impl<> BaseLen for StreamReceiverConfigGen<> {
    const BASE_LEN: usize = 0;
}

impl<> Encode for StreamReceiverConfigGen<> {
    fn scratch_len(&self) -> usize {
        0
    }

    fn encode(&self, _: &mut EncodeCursor) {}
}

impl Owned for StreamReceiverConfig {
    type Lazy<'a> = StreamReceiverConfigLazy<'a>;

    fn lazy_to_owned(lazy: Self::Lazy<'_>) -> DecodeResult<Self> {
        TryFrom::try_from(lazy)
    }
}

impl<'a> Lazy<'a> for StreamReceiverConfigLazy<'a> {
    type Owned = StreamReceiverConfig;
}

impl<'a> Compatible<StreamReceiverConfigLazy<'a>> for StreamReceiverConfig { }
impl<'a> Compatible<StreamReceiverConfig> for StreamReceiverConfigLazy<'a> { }

impl<'a> StreamReceiverConfigLazy<'a> {}

impl BaseLen for StreamReceiverConfig {
    const BASE_LEN: usize = 0;
}

impl Encode for StreamReceiverConfig {
    fn scratch_len(&self) -> usize {
        0
    }

    fn encode(&self, _: &mut EncodeCursor) {}
}

impl<'a> Decode<'a> for StreamReceiverConfig {
    fn decode(_: &DecodeCursor<'a>) -> DecodeResult<Self> {

        Ok(StreamReceiverConfig {})
    }
}

impl<'a> BaseLen for StreamReceiverConfigLazy<'a> {
    const BASE_LEN: usize = 0;
}

impl<'a> Encode for StreamReceiverConfigLazy<'a> {
    fn scratch_len(&self) -> usize {
        0
    }

    fn encode(&self, _: &mut EncodeCursor) {}
}

impl<'a> Decode<'a> for StreamReceiverConfigLazy<'a> {
    fn decode(cursor: &DecodeCursor<'a>) -> DecodeResult<Self> {
        let offset = cursor.offset();
        cursor.advance(Self::BASE_LEN);
        Ok(StreamReceiverConfigLazy {
            buffer: cursor.buffer(),
            offset,
        })
    }
}

impl<'a> TryFrom<StreamReceiverConfigLazy<'a>> for StreamReceiverConfig {
    type Error = DecodeError;

    fn try_from(other: StreamReceiverConfigLazy<'a>) -> Result<Self, Self::Error> {
        let cursor = DecodeCursor::at_offset(other.buffer, other.offset);
        Decode::decode(&cursor)
    }
}

impl<'a> Copy for StreamReceiverConfigLazy<'a> { }

impl<'a> Clone for StreamReceiverConfigLazy<'a> {
    fn clone(&self) -> Self {
        Self {
            buffer: self.buffer,
            offset: self.offset,
        }
    }
}

impl<'a> core::fmt::Debug for StreamReceiverConfigLazy<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("StreamReceiverConfigLazy")
            .finish()
    }
}

impl<'a> PartialEq for StreamReceiverConfigLazy<'a> {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Default)]
pub struct StreamSenderConfig {}

pub struct StreamSenderConfigLazy<'a> {
    buffer: &'a [u8],
    offset: usize,
}

pub struct StreamSenderConfigGen<> {}

impl<> Compatible<StreamSenderConfig> for StreamSenderConfigGen<> { }
impl<> Compatible<StreamSenderConfigGen<>> for StreamSenderConfig { }

impl<> BaseLen for StreamSenderConfigGen<> {
    const BASE_LEN: usize = 0;
}

impl<> Encode for StreamSenderConfigGen<> {
    fn scratch_len(&self) -> usize {
        0
    }

    fn encode(&self, _: &mut EncodeCursor) {}
}

impl Owned for StreamSenderConfig {
    type Lazy<'a> = StreamSenderConfigLazy<'a>;

    fn lazy_to_owned(lazy: Self::Lazy<'_>) -> DecodeResult<Self> {
        TryFrom::try_from(lazy)
    }
}

impl<'a> Lazy<'a> for StreamSenderConfigLazy<'a> {
    type Owned = StreamSenderConfig;
}

impl<'a> Compatible<StreamSenderConfigLazy<'a>> for StreamSenderConfig { }
impl<'a> Compatible<StreamSenderConfig> for StreamSenderConfigLazy<'a> { }

impl<'a> StreamSenderConfigLazy<'a> {}

impl BaseLen for StreamSenderConfig {
    const BASE_LEN: usize = 0;
}

impl Encode for StreamSenderConfig {
    fn scratch_len(&self) -> usize {
        0
    }

    fn encode(&self, _: &mut EncodeCursor) {}
}

impl<'a> Decode<'a> for StreamSenderConfig {
    fn decode(_: &DecodeCursor<'a>) -> DecodeResult<Self> {

        Ok(StreamSenderConfig {})
    }
}

impl<'a> BaseLen for StreamSenderConfigLazy<'a> {
    const BASE_LEN: usize = 0;
}

impl<'a> Encode for StreamSenderConfigLazy<'a> {
    fn scratch_len(&self) -> usize {
        0
    }

    fn encode(&self, _: &mut EncodeCursor) {}
}

impl<'a> Decode<'a> for StreamSenderConfigLazy<'a> {
    fn decode(cursor: &DecodeCursor<'a>) -> DecodeResult<Self> {
        let offset = cursor.offset();
        cursor.advance(Self::BASE_LEN);
        Ok(StreamSenderConfigLazy {
            buffer: cursor.buffer(),
            offset,
        })
    }
}

impl<'a> TryFrom<StreamSenderConfigLazy<'a>> for StreamSenderConfig {
    type Error = DecodeError;

    fn try_from(other: StreamSenderConfigLazy<'a>) -> Result<Self, Self::Error> {
        let cursor = DecodeCursor::at_offset(other.buffer, other.offset);
        Decode::decode(&cursor)
    }
}

impl<'a> Copy for StreamSenderConfigLazy<'a> { }

impl<'a> Clone for StreamSenderConfigLazy<'a> {
    fn clone(&self) -> Self {
        Self {
            buffer: self.buffer,
            offset: self.offset,
        }
    }
}

impl<'a> core::fmt::Debug for StreamSenderConfigLazy<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("StreamSenderConfigLazy")
            .finish()
    }
}

impl<'a> PartialEq for StreamSenderConfigLazy<'a> {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Default)]
pub struct MultiStreamInitState {}

pub struct MultiStreamInitStateLazy<'a> {
    buffer: &'a [u8],
    offset: usize,
}

pub struct MultiStreamInitStateGen<> {}

impl<> Compatible<MultiStreamInitState> for MultiStreamInitStateGen<> { }
impl<> Compatible<MultiStreamInitStateGen<>> for MultiStreamInitState { }

impl<> BaseLen for MultiStreamInitStateGen<> {
    const BASE_LEN: usize = 0;
}

impl<> Encode for MultiStreamInitStateGen<> {
    fn scratch_len(&self) -> usize {
        0
    }

    fn encode(&self, _: &mut EncodeCursor) {}
}

impl Owned for MultiStreamInitState {
    type Lazy<'a> = MultiStreamInitStateLazy<'a>;

    fn lazy_to_owned(lazy: Self::Lazy<'_>) -> DecodeResult<Self> {
        TryFrom::try_from(lazy)
    }
}

impl<'a> Lazy<'a> for MultiStreamInitStateLazy<'a> {
    type Owned = MultiStreamInitState;
}

impl<'a> Compatible<MultiStreamInitStateLazy<'a>> for MultiStreamInitState { }
impl<'a> Compatible<MultiStreamInitState> for MultiStreamInitStateLazy<'a> { }

impl<'a> MultiStreamInitStateLazy<'a> {}

impl BaseLen for MultiStreamInitState {
    const BASE_LEN: usize = 0;
}

impl Encode for MultiStreamInitState {
    fn scratch_len(&self) -> usize {
        0
    }

    fn encode(&self, _: &mut EncodeCursor) {}
}

impl<'a> Decode<'a> for MultiStreamInitState {
    fn decode(_: &DecodeCursor<'a>) -> DecodeResult<Self> {

        Ok(MultiStreamInitState {})
    }
}

impl<'a> BaseLen for MultiStreamInitStateLazy<'a> {
    const BASE_LEN: usize = 0;
}

impl<'a> Encode for MultiStreamInitStateLazy<'a> {
    fn scratch_len(&self) -> usize {
        0
    }

    fn encode(&self, _: &mut EncodeCursor) {}
}

impl<'a> Decode<'a> for MultiStreamInitStateLazy<'a> {
    fn decode(cursor: &DecodeCursor<'a>) -> DecodeResult<Self> {
        let offset = cursor.offset();
        cursor.advance(Self::BASE_LEN);
        Ok(MultiStreamInitStateLazy {
            buffer: cursor.buffer(),
            offset,
        })
    }
}

impl<'a> TryFrom<MultiStreamInitStateLazy<'a>> for MultiStreamInitState {
    type Error = DecodeError;

    fn try_from(other: MultiStreamInitStateLazy<'a>) -> Result<Self, Self::Error> {
        let cursor = DecodeCursor::at_offset(other.buffer, other.offset);
        Decode::decode(&cursor)
    }
}

impl<'a> Copy for MultiStreamInitStateLazy<'a> { }

impl<'a> Clone for MultiStreamInitStateLazy<'a> {
    fn clone(&self) -> Self {
        Self {
            buffer: self.buffer,
            offset: self.offset,
        }
    }
}

impl<'a> core::fmt::Debug for MultiStreamInitStateLazy<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("MultiStreamInitStateLazy")
            .finish()
    }
}

impl<'a> PartialEq for MultiStreamInitStateLazy<'a> {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Default)]
pub struct MultiStreamReceiverConfig {}

pub struct MultiStreamReceiverConfigLazy<'a> {
    buffer: &'a [u8],
    offset: usize,
}

pub struct MultiStreamReceiverConfigGen<> {}

impl<> Compatible<MultiStreamReceiverConfig> for MultiStreamReceiverConfigGen<> { }
impl<> Compatible<MultiStreamReceiverConfigGen<>> for MultiStreamReceiverConfig { }

impl<> BaseLen for MultiStreamReceiverConfigGen<> {
    const BASE_LEN: usize = 0;
}

impl<> Encode for MultiStreamReceiverConfigGen<> {
    fn scratch_len(&self) -> usize {
        0
    }

    fn encode(&self, _: &mut EncodeCursor) {}
}

impl Owned for MultiStreamReceiverConfig {
    type Lazy<'a> = MultiStreamReceiverConfigLazy<'a>;

    fn lazy_to_owned(lazy: Self::Lazy<'_>) -> DecodeResult<Self> {
        TryFrom::try_from(lazy)
    }
}

impl<'a> Lazy<'a> for MultiStreamReceiverConfigLazy<'a> {
    type Owned = MultiStreamReceiverConfig;
}

impl<'a> Compatible<MultiStreamReceiverConfigLazy<'a>> for MultiStreamReceiverConfig { }
impl<'a> Compatible<MultiStreamReceiverConfig> for MultiStreamReceiverConfigLazy<'a> { }

impl<'a> MultiStreamReceiverConfigLazy<'a> {}

impl BaseLen for MultiStreamReceiverConfig {
    const BASE_LEN: usize = 0;
}

impl Encode for MultiStreamReceiverConfig {
    fn scratch_len(&self) -> usize {
        0
    }

    fn encode(&self, _: &mut EncodeCursor) {}
}

impl<'a> Decode<'a> for MultiStreamReceiverConfig {
    fn decode(_: &DecodeCursor<'a>) -> DecodeResult<Self> {

        Ok(MultiStreamReceiverConfig {})
    }
}

impl<'a> BaseLen for MultiStreamReceiverConfigLazy<'a> {
    const BASE_LEN: usize = 0;
}

impl<'a> Encode for MultiStreamReceiverConfigLazy<'a> {
    fn scratch_len(&self) -> usize {
        0
    }

    fn encode(&self, _: &mut EncodeCursor) {}
}

impl<'a> Decode<'a> for MultiStreamReceiverConfigLazy<'a> {
    fn decode(cursor: &DecodeCursor<'a>) -> DecodeResult<Self> {
        let offset = cursor.offset();
        cursor.advance(Self::BASE_LEN);
        Ok(MultiStreamReceiverConfigLazy {
            buffer: cursor.buffer(),
            offset,
        })
    }
}

impl<'a> TryFrom<MultiStreamReceiverConfigLazy<'a>> for MultiStreamReceiverConfig {
    type Error = DecodeError;

    fn try_from(other: MultiStreamReceiverConfigLazy<'a>) -> Result<Self, Self::Error> {
        let cursor = DecodeCursor::at_offset(other.buffer, other.offset);
        Decode::decode(&cursor)
    }
}

impl<'a> Copy for MultiStreamReceiverConfigLazy<'a> { }

impl<'a> Clone for MultiStreamReceiverConfigLazy<'a> {
    fn clone(&self) -> Self {
        Self {
            buffer: self.buffer,
            offset: self.offset,
        }
    }
}

impl<'a> core::fmt::Debug for MultiStreamReceiverConfigLazy<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("MultiStreamReceiverConfigLazy")
            .finish()
    }
}

impl<'a> PartialEq for MultiStreamReceiverConfigLazy<'a> {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Default)]
pub struct MultiStreamSenderConfig {}

pub struct MultiStreamSenderConfigLazy<'a> {
    buffer: &'a [u8],
    offset: usize,
}

pub struct MultiStreamSenderConfigGen<> {}

impl<> Compatible<MultiStreamSenderConfig> for MultiStreamSenderConfigGen<> { }
impl<> Compatible<MultiStreamSenderConfigGen<>> for MultiStreamSenderConfig { }

impl<> BaseLen for MultiStreamSenderConfigGen<> {
    const BASE_LEN: usize = 0;
}

impl<> Encode for MultiStreamSenderConfigGen<> {
    fn scratch_len(&self) -> usize {
        0
    }

    fn encode(&self, _: &mut EncodeCursor) {}
}

impl Owned for MultiStreamSenderConfig {
    type Lazy<'a> = MultiStreamSenderConfigLazy<'a>;

    fn lazy_to_owned(lazy: Self::Lazy<'_>) -> DecodeResult<Self> {
        TryFrom::try_from(lazy)
    }
}

impl<'a> Lazy<'a> for MultiStreamSenderConfigLazy<'a> {
    type Owned = MultiStreamSenderConfig;
}

impl<'a> Compatible<MultiStreamSenderConfigLazy<'a>> for MultiStreamSenderConfig { }
impl<'a> Compatible<MultiStreamSenderConfig> for MultiStreamSenderConfigLazy<'a> { }

impl<'a> MultiStreamSenderConfigLazy<'a> {}

impl BaseLen for MultiStreamSenderConfig {
    const BASE_LEN: usize = 0;
}

impl Encode for MultiStreamSenderConfig {
    fn scratch_len(&self) -> usize {
        0
    }

    fn encode(&self, _: &mut EncodeCursor) {}
}

impl<'a> Decode<'a> for MultiStreamSenderConfig {
    fn decode(_: &DecodeCursor<'a>) -> DecodeResult<Self> {

        Ok(MultiStreamSenderConfig {})
    }
}

impl<'a> BaseLen for MultiStreamSenderConfigLazy<'a> {
    const BASE_LEN: usize = 0;
}

impl<'a> Encode for MultiStreamSenderConfigLazy<'a> {
    fn scratch_len(&self) -> usize {
        0
    }

    fn encode(&self, _: &mut EncodeCursor) {}
}

impl<'a> Decode<'a> for MultiStreamSenderConfigLazy<'a> {
    fn decode(cursor: &DecodeCursor<'a>) -> DecodeResult<Self> {
        let offset = cursor.offset();
        cursor.advance(Self::BASE_LEN);
        Ok(MultiStreamSenderConfigLazy {
            buffer: cursor.buffer(),
            offset,
        })
    }
}

impl<'a> TryFrom<MultiStreamSenderConfigLazy<'a>> for MultiStreamSenderConfig {
    type Error = DecodeError;

    fn try_from(other: MultiStreamSenderConfigLazy<'a>) -> Result<Self, Self::Error> {
        let cursor = DecodeCursor::at_offset(other.buffer, other.offset);
        Decode::decode(&cursor)
    }
}

impl<'a> Copy for MultiStreamSenderConfigLazy<'a> { }

impl<'a> Clone for MultiStreamSenderConfigLazy<'a> {
    fn clone(&self) -> Self {
        Self {
            buffer: self.buffer,
            offset: self.offset,
        }
    }
}

impl<'a> core::fmt::Debug for MultiStreamSenderConfigLazy<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("MultiStreamSenderConfigLazy")
            .finish()
    }
}

impl<'a> PartialEq for MultiStreamSenderConfigLazy<'a> {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Default)]
pub struct ByteStreamInitState {}

pub struct ByteStreamInitStateLazy<'a> {
    buffer: &'a [u8],
    offset: usize,
}

pub struct ByteStreamInitStateGen<> {}

impl<> Compatible<ByteStreamInitState> for ByteStreamInitStateGen<> { }
impl<> Compatible<ByteStreamInitStateGen<>> for ByteStreamInitState { }

impl<> BaseLen for ByteStreamInitStateGen<> {
    const BASE_LEN: usize = 0;
}

impl<> Encode for ByteStreamInitStateGen<> {
    fn scratch_len(&self) -> usize {
        0
    }

    fn encode(&self, _: &mut EncodeCursor) {}
}

impl Owned for ByteStreamInitState {
    type Lazy<'a> = ByteStreamInitStateLazy<'a>;

    fn lazy_to_owned(lazy: Self::Lazy<'_>) -> DecodeResult<Self> {
        TryFrom::try_from(lazy)
    }
}

impl<'a> Lazy<'a> for ByteStreamInitStateLazy<'a> {
    type Owned = ByteStreamInitState;
}

impl<'a> Compatible<ByteStreamInitStateLazy<'a>> for ByteStreamInitState { }
impl<'a> Compatible<ByteStreamInitState> for ByteStreamInitStateLazy<'a> { }

impl<'a> ByteStreamInitStateLazy<'a> {}

impl BaseLen for ByteStreamInitState {
    const BASE_LEN: usize = 0;
}

impl Encode for ByteStreamInitState {
    fn scratch_len(&self) -> usize {
        0
    }

    fn encode(&self, _: &mut EncodeCursor) {}
}

impl<'a> Decode<'a> for ByteStreamInitState {
    fn decode(_: &DecodeCursor<'a>) -> DecodeResult<Self> {

        Ok(ByteStreamInitState {})
    }
}

impl<'a> BaseLen for ByteStreamInitStateLazy<'a> {
    const BASE_LEN: usize = 0;
}

impl<'a> Encode for ByteStreamInitStateLazy<'a> {
    fn scratch_len(&self) -> usize {
        0
    }

    fn encode(&self, _: &mut EncodeCursor) {}
}

impl<'a> Decode<'a> for ByteStreamInitStateLazy<'a> {
    fn decode(cursor: &DecodeCursor<'a>) -> DecodeResult<Self> {
        let offset = cursor.offset();
        cursor.advance(Self::BASE_LEN);
        Ok(ByteStreamInitStateLazy {
            buffer: cursor.buffer(),
            offset,
        })
    }
}

impl<'a> TryFrom<ByteStreamInitStateLazy<'a>> for ByteStreamInitState {
    type Error = DecodeError;

    fn try_from(other: ByteStreamInitStateLazy<'a>) -> Result<Self, Self::Error> {
        let cursor = DecodeCursor::at_offset(other.buffer, other.offset);
        Decode::decode(&cursor)
    }
}

impl<'a> Copy for ByteStreamInitStateLazy<'a> { }

impl<'a> Clone for ByteStreamInitStateLazy<'a> {
    fn clone(&self) -> Self {
        Self {
            buffer: self.buffer,
            offset: self.offset,
        }
    }
}

impl<'a> core::fmt::Debug for ByteStreamInitStateLazy<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ByteStreamInitStateLazy")
            .finish()
    }
}

impl<'a> PartialEq for ByteStreamInitStateLazy<'a> {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Default)]
pub struct ByteStreamReceiverConfig {}

pub struct ByteStreamReceiverConfigLazy<'a> {
    buffer: &'a [u8],
    offset: usize,
}

pub struct ByteStreamReceiverConfigGen<> {}

impl<> Compatible<ByteStreamReceiverConfig> for ByteStreamReceiverConfigGen<> { }
impl<> Compatible<ByteStreamReceiverConfigGen<>> for ByteStreamReceiverConfig { }

impl<> BaseLen for ByteStreamReceiverConfigGen<> {
    const BASE_LEN: usize = 0;
}

impl<> Encode for ByteStreamReceiverConfigGen<> {
    fn scratch_len(&self) -> usize {
        0
    }

    fn encode(&self, _: &mut EncodeCursor) {}
}

impl Owned for ByteStreamReceiverConfig {
    type Lazy<'a> = ByteStreamReceiverConfigLazy<'a>;

    fn lazy_to_owned(lazy: Self::Lazy<'_>) -> DecodeResult<Self> {
        TryFrom::try_from(lazy)
    }
}

impl<'a> Lazy<'a> for ByteStreamReceiverConfigLazy<'a> {
    type Owned = ByteStreamReceiverConfig;
}

impl<'a> Compatible<ByteStreamReceiverConfigLazy<'a>> for ByteStreamReceiverConfig { }
impl<'a> Compatible<ByteStreamReceiverConfig> for ByteStreamReceiverConfigLazy<'a> { }

impl<'a> ByteStreamReceiverConfigLazy<'a> {}

impl BaseLen for ByteStreamReceiverConfig {
    const BASE_LEN: usize = 0;
}

impl Encode for ByteStreamReceiverConfig {
    fn scratch_len(&self) -> usize {
        0
    }

    fn encode(&self, _: &mut EncodeCursor) {}
}

impl<'a> Decode<'a> for ByteStreamReceiverConfig {
    fn decode(_: &DecodeCursor<'a>) -> DecodeResult<Self> {

        Ok(ByteStreamReceiverConfig {})
    }
}

impl<'a> BaseLen for ByteStreamReceiverConfigLazy<'a> {
    const BASE_LEN: usize = 0;
}

impl<'a> Encode for ByteStreamReceiverConfigLazy<'a> {
    fn scratch_len(&self) -> usize {
        0
    }

    fn encode(&self, _: &mut EncodeCursor) {}
}

impl<'a> Decode<'a> for ByteStreamReceiverConfigLazy<'a> {
    fn decode(cursor: &DecodeCursor<'a>) -> DecodeResult<Self> {
        let offset = cursor.offset();
        cursor.advance(Self::BASE_LEN);
        Ok(ByteStreamReceiverConfigLazy {
            buffer: cursor.buffer(),
            offset,
        })
    }
}

impl<'a> TryFrom<ByteStreamReceiverConfigLazy<'a>> for ByteStreamReceiverConfig {
    type Error = DecodeError;

    fn try_from(other: ByteStreamReceiverConfigLazy<'a>) -> Result<Self, Self::Error> {
        let cursor = DecodeCursor::at_offset(other.buffer, other.offset);
        Decode::decode(&cursor)
    }
}

impl<'a> Copy for ByteStreamReceiverConfigLazy<'a> { }

impl<'a> Clone for ByteStreamReceiverConfigLazy<'a> {
    fn clone(&self) -> Self {
        Self {
            buffer: self.buffer,
            offset: self.offset,
        }
    }
}

impl<'a> core::fmt::Debug for ByteStreamReceiverConfigLazy<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ByteStreamReceiverConfigLazy")
            .finish()
    }
}

impl<'a> PartialEq for ByteStreamReceiverConfigLazy<'a> {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Default)]
pub struct ByteStreamSenderConfig {}

pub struct ByteStreamSenderConfigLazy<'a> {
    buffer: &'a [u8],
    offset: usize,
}

pub struct ByteStreamSenderConfigGen<> {}

impl<> Compatible<ByteStreamSenderConfig> for ByteStreamSenderConfigGen<> { }
impl<> Compatible<ByteStreamSenderConfigGen<>> for ByteStreamSenderConfig { }

impl<> BaseLen for ByteStreamSenderConfigGen<> {
    const BASE_LEN: usize = 0;
}

impl<> Encode for ByteStreamSenderConfigGen<> {
    fn scratch_len(&self) -> usize {
        0
    }

    fn encode(&self, _: &mut EncodeCursor) {}
}

impl Owned for ByteStreamSenderConfig {
    type Lazy<'a> = ByteStreamSenderConfigLazy<'a>;

    fn lazy_to_owned(lazy: Self::Lazy<'_>) -> DecodeResult<Self> {
        TryFrom::try_from(lazy)
    }
}

impl<'a> Lazy<'a> for ByteStreamSenderConfigLazy<'a> {
    type Owned = ByteStreamSenderConfig;
}

impl<'a> Compatible<ByteStreamSenderConfigLazy<'a>> for ByteStreamSenderConfig { }
impl<'a> Compatible<ByteStreamSenderConfig> for ByteStreamSenderConfigLazy<'a> { }

impl<'a> ByteStreamSenderConfigLazy<'a> {}

impl BaseLen for ByteStreamSenderConfig {
    const BASE_LEN: usize = 0;
}

impl Encode for ByteStreamSenderConfig {
    fn scratch_len(&self) -> usize {
        0
    }

    fn encode(&self, _: &mut EncodeCursor) {}
}

impl<'a> Decode<'a> for ByteStreamSenderConfig {
    fn decode(_: &DecodeCursor<'a>) -> DecodeResult<Self> {

        Ok(ByteStreamSenderConfig {})
    }
}

impl<'a> BaseLen for ByteStreamSenderConfigLazy<'a> {
    const BASE_LEN: usize = 0;
}

impl<'a> Encode for ByteStreamSenderConfigLazy<'a> {
    fn scratch_len(&self) -> usize {
        0
    }

    fn encode(&self, _: &mut EncodeCursor) {}
}

impl<'a> Decode<'a> for ByteStreamSenderConfigLazy<'a> {
    fn decode(cursor: &DecodeCursor<'a>) -> DecodeResult<Self> {
        let offset = cursor.offset();
        cursor.advance(Self::BASE_LEN);
        Ok(ByteStreamSenderConfigLazy {
            buffer: cursor.buffer(),
            offset,
        })
    }
}

impl<'a> TryFrom<ByteStreamSenderConfigLazy<'a>> for ByteStreamSenderConfig {
    type Error = DecodeError;

    fn try_from(other: ByteStreamSenderConfigLazy<'a>) -> Result<Self, Self::Error> {
        let cursor = DecodeCursor::at_offset(other.buffer, other.offset);
        Decode::decode(&cursor)
    }
}

impl<'a> Copy for ByteStreamSenderConfigLazy<'a> { }

impl<'a> Clone for ByteStreamSenderConfigLazy<'a> {
    fn clone(&self) -> Self {
        Self {
            buffer: self.buffer,
            offset: self.offset,
        }
    }
}

impl<'a> core::fmt::Debug for ByteStreamSenderConfigLazy<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ByteStreamSenderConfigLazy")
            .finish()
    }
}

impl<'a> PartialEq for ByteStreamSenderConfigLazy<'a> {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}
