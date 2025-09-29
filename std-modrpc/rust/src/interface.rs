use crate::proto::{MultiStreamItem, PropertyUpdate, Request, Response, StreamItem};
use modrpc::{InterfaceBuilder, InterfaceEvent, InterfaceSchema};

pub struct PropertyInterface<T> {
    pub update: InterfaceEvent<PropertyUpdate<T>>,
}

impl<T> InterfaceSchema for PropertyInterface<T> {
    fn new(ib: &mut InterfaceBuilder) -> Self {
        Self {
            update: ib.event("update"),
        }
    }
}

pub struct RequestInterface<Req, Resp> {
    pub request: InterfaceEvent<Request<Req>>,
    pub response: InterfaceEvent<Response<Resp>>,
}

impl<Req, Resp> InterfaceSchema for RequestInterface<Req, Resp> {
    fn new(ib: &mut InterfaceBuilder) -> Self {
        Self {
            request: ib.event("request"),
            response: ib.event("response"),
        }
    }
}

pub struct StreamInterface<T> {
    pub item: InterfaceEvent<StreamItem<T>>,
}

impl<T> InterfaceSchema for StreamInterface<T> {
    fn new(ib: &mut InterfaceBuilder) -> Self {
        Self {
            item: ib.event("item"),
        }
    }
}

pub struct MultiStreamInterface<T> {
    pub item: InterfaceEvent<MultiStreamItem<T>>,
}

impl<T> InterfaceSchema for MultiStreamInterface<T> {
    fn new(ib: &mut InterfaceBuilder) -> Self {
        Self {
            item: ib.event("item"),
        }
    }
}

pub struct ByteStreamInterface {
    pub blob: InterfaceEvent<()>,
}

impl InterfaceSchema for ByteStreamInterface {
    fn new(ib: &mut InterfaceBuilder) -> Self {
        Self {
            blob: ib.event("blob"),
        }
    }
}
