/*
 * Copyright 2018 Bitwise IO, Inc.
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 * -----------------------------------------------------------------------------
 */
//! The `receipts` module contains structs that supply information on the processing
//! of `Transaction`s
use protobuf::Message;

use crate::protos;
use crate::protos::{
    FromBytes, FromNative, FromProto, IntoBytes, IntoNative, IntoProto, ProtoConversionError,
};
use crate::state;
use std::error::Error as StdError;
use std::fmt;

/// A change to be applied to state, in terms of keys and values.
///
/// A `StateChange` represents the basic level of changes that can be applied to
/// values in state.  This covers the setting of a key/value pair, or the
/// deletion of a key.
#[derive(Eq, Hash, PartialEq)]
pub enum StateChange {
    Set { key: String, value: Vec<u8> },
    Delete { key: String },
}

impl StateChange {
    /// Compares StateChanges based on the key, regardless of variant
    pub fn has_key(&self, k: &str) -> bool {
        if let StateChange::Set { key, .. } = self {
            key == k
        } else if let StateChange::Delete { key } = self {
            key == k
        } else {
            false
        }
    }
}

impl Into<state::StateChange> for StateChange {
    /// Converts this modules StateChange into state::StateChange enum
    fn into(self) -> state::StateChange {
        match self {
            StateChange::Set { key, value } => state::StateChange::Set { key, value },
            StateChange::Delete { key } => state::StateChange::Delete { key },
        }
    }
}

impl fmt::Debug for StateChange {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            StateChange::Set { key, value } => {
                f.write_str("StateChange{ ")?;
                write!(f, "key: {:?}, ", key)?;
                let value_len = value.len();
                write!(
                    f,
                    "value: <{} byte{}>",
                    value_len,
                    if value_len == 1 { "" } else { "s" }
                )?;
                f.write_str(" }")
            }
            StateChange::Delete { key } => write!(f, "StateChange::Delete{{ key: {:?} }})", key),
        }
    }
}

impl Clone for StateChange {
    fn clone(&self) -> Self {
        match self {
            StateChange::Set { key, value } => StateChange::Set {
                key: key.clone(),
                value: value.clone(),
            },
            StateChange::Delete { key } => StateChange::Delete { key: key.clone() },
        }
    }
}

impl FromProto<protos::transaction_receipt::StateChange> for StateChange {
    fn from_proto(
        state_change: protos::transaction_receipt::StateChange,
    ) -> Result<Self, ProtoConversionError> {
        match state_change.get_field_type() {
            protos::transaction_receipt::StateChange_Type::SET => Ok(StateChange::Set {
                key: state_change.get_address().to_string(),
                value: state_change.get_value().to_vec(),
            }),
            protos::transaction_receipt::StateChange_Type::DELETE => Ok(StateChange::Delete {
                key: state_change.get_address().to_string(),
            }),
            protos::transaction_receipt::StateChange_Type::TYPE_UNSET => {
                Err(ProtoConversionError::InvalidTypeError(
                    "Cannot convert StateChange with type unset. /
                    StageChange type must be StateChange_Type::SET or StateChange_Type::DELETE."
                        .to_string(),
                ))
            }
        }
    }
}

impl FromNative<StateChange> for protos::transaction_receipt::StateChange {
    fn from_native(state_change: StateChange) -> Result<Self, ProtoConversionError> {
        let mut proto_state_change = protos::transaction_receipt::StateChange::new();

        match state_change {
            StateChange::Set { key, value } => {
                proto_state_change.set_value(value);
                proto_state_change.set_address(key);
                proto_state_change
                    .set_field_type(protos::transaction_receipt::StateChange_Type::SET);
                Ok(proto_state_change)
            }
            StateChange::Delete { key } => {
                proto_state_change.set_address(key);
                proto_state_change
                    .set_field_type(protos::transaction_receipt::StateChange_Type::DELETE);
                Ok(proto_state_change)
            }
        }
    }
}

impl FromBytes<StateChange> for StateChange {
    fn from_bytes(bytes: &[u8]) -> Result<StateChange, ProtoConversionError> {
        let proto: protos::transaction_receipt::StateChange = protobuf::parse_from_bytes(bytes)
            .map_err(|_| {
                ProtoConversionError::SerializationError(
                    "Unable to get StateChange from bytes".to_string(),
                )
            })?;
        proto.into_native()
    }
}

impl IntoBytes for StateChange {
    fn into_bytes(self) -> Result<Vec<u8>, ProtoConversionError> {
        let proto = self.into_proto()?;
        let bytes = proto.write_to_bytes().map_err(|_| {
            ProtoConversionError::SerializationError(
                "Unable to get bytes from StateChanger".to_string(),
            )
        })?;
        Ok(bytes)
    }
}

impl IntoProto<protos::transaction_receipt::StateChange> for StateChange {}
impl IntoNative<StateChange> for protos::transaction_receipt::StateChange {}

/// A `TransactionReceipt` has the state changes associated with a valid transaction.
#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub struct TransactionReceipt {
    /// Updates to state that were generated by the transaction.
    pub state_changes: Vec<StateChange>,
    /// Events fired by this transaction.
    pub events: Vec<Event>,
    /// Transaction family defined data.
    pub data: Vec<Vec<u8>>,

    pub transaction_id: String,
}

impl FromProto<protos::transaction_receipt::TransactionReceipt> for TransactionReceipt {
    fn from_proto(
        transaction_receipt: protos::transaction_receipt::TransactionReceipt,
    ) -> Result<Self, ProtoConversionError> {
        Ok(TransactionReceipt {
            state_changes: transaction_receipt
                .get_state_changes()
                .to_vec()
                .into_iter()
                .map(StateChange::from_proto)
                .collect::<Result<Vec<StateChange>, ProtoConversionError>>()?,
            events: transaction_receipt
                .get_events()
                .to_vec()
                .into_iter()
                .map(Event::from_proto)
                .collect::<Result<Vec<Event>, ProtoConversionError>>()?,
            data: transaction_receipt.get_data().to_vec(),
            transaction_id: transaction_receipt.get_transaction_id().to_string(),
        })
    }
}

impl FromNative<TransactionReceipt> for protos::transaction_receipt::TransactionReceipt {
    fn from_native(transaction_receipt: TransactionReceipt) -> Result<Self, ProtoConversionError> {
        let mut proto_transaction_receipt = protos::transaction_receipt::TransactionReceipt::new();
        proto_transaction_receipt.set_state_changes(
            protobuf::RepeatedField::from_vec(transaction_receipt
                .state_changes
                .into_iter()
                .map(protos::transaction_receipt::StateChange::from_native)
                .collect::<Result<Vec<protos::transaction_receipt::StateChange>, ProtoConversionError>>()?,
        ));
        proto_transaction_receipt.set_events(protobuf::RepeatedField::from_vec(
            transaction_receipt
                .events
                .into_iter()
                .map(protos::events::Event::from_native)
                .collect::<Result<Vec<protos::events::Event>, ProtoConversionError>>()?,
        ));
        proto_transaction_receipt
            .set_data(protobuf::RepeatedField::from_vec(transaction_receipt.data));
        proto_transaction_receipt.set_transaction_id(transaction_receipt.transaction_id);
        Ok(proto_transaction_receipt)
    }
}

impl FromBytes<TransactionReceipt> for TransactionReceipt {
    fn from_bytes(bytes: &[u8]) -> Result<TransactionReceipt, ProtoConversionError> {
        let proto: protos::transaction_receipt::TransactionReceipt =
            protobuf::parse_from_bytes(bytes).map_err(|_| {
                ProtoConversionError::SerializationError(
                    "Unable to get TransactionReceipt from bytes".to_string(),
                )
            })?;
        proto.into_native()
    }
}

impl IntoBytes for TransactionReceipt {
    fn into_bytes(self) -> Result<Vec<u8>, ProtoConversionError> {
        let proto = self.into_proto()?;
        let bytes = proto.write_to_bytes().map_err(|_| {
            ProtoConversionError::SerializationError(
                "Unable to get bytes from TransactionReceipt".to_string(),
            )
        })?;
        Ok(bytes)
    }
}

impl IntoProto<protos::transaction_receipt::TransactionReceipt> for TransactionReceipt {}
impl IntoNative<TransactionReceipt> for protos::transaction_receipt::TransactionReceipt {}

/// An `Event` is metadata about a `Transaction`'s processing. Events are
/// transmitted by `transact` but are not verified or saved to state. `Event`s are
/// generated by any of the ways that `Transaction`s are processed in `transact`.
#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub struct Event {
    /// A human readable string that identifies this event
    /// Determined by the client.
    pub event_type: String,
    /// Human readable information about the transaction.
    pub attributes: Vec<(String, String)>,
    /// Opaque data, about the transaction, that the client can deserialize
    pub data: Vec<u8>,
}

impl FromProto<protos::events::Event> for Event {
    fn from_proto(event: protos::events::Event) -> Result<Self, ProtoConversionError> {
        Ok(Event {
            event_type: event.get_event_type().to_string(),
            attributes: event
                .get_attributes()
                .to_vec()
                .into_iter()
                .map(|attr| (attr.get_key().to_string(), attr.get_value().to_string()))
                .collect(),
            data: event.get_data().to_vec(),
        })
    }
}

impl FromNative<Event> for protos::events::Event {
    fn from_native(event: Event) -> Result<Self, ProtoConversionError> {
        let mut proto_event = protos::events::Event::new();
        proto_event.set_event_type(event.event_type);
        proto_event.set_attributes(
            event
                .attributes
                .iter()
                .map(|(key, value)| {
                    let mut event_attr = protos::events::Event_Attribute::new();
                    event_attr.set_key(key.to_string());
                    event_attr.set_value(value.to_string());
                    event_attr
                })
                .collect::<protobuf::RepeatedField<protos::events::Event_Attribute>>(),
        );
        proto_event.set_data(event.data);
        Ok(proto_event)
    }
}

impl FromBytes<Event> for Event {
    fn from_bytes(bytes: &[u8]) -> Result<Event, ProtoConversionError> {
        let proto: protos::events::Event = protobuf::parse_from_bytes(bytes).map_err(|_| {
            ProtoConversionError::SerializationError(
                "Unable to get TransactionReceipt from bytes".to_string(),
            )
        })?;
        proto.into_native()
    }
}

impl IntoBytes for Event {
    fn into_bytes(self) -> Result<Vec<u8>, ProtoConversionError> {
        let proto = self.into_proto()?;
        let bytes = proto.write_to_bytes().map_err(|_| {
            ProtoConversionError::SerializationError("Unable to get bytes from Event".to_string())
        })?;
        Ok(bytes)
    }
}

impl IntoProto<protos::events::Event> for Event {}
impl IntoNative<Event> for protos::events::Event {}

#[derive(Debug)]
pub enum EventBuilderError {
    MissingField(String),
}

impl StdError for EventBuilderError {
    fn description(&self) -> &str {
        match *self {
            EventBuilderError::MissingField(ref msg) => msg,
        }
    }

    fn cause(&self) -> Option<&dyn StdError> {
        match *self {
            EventBuilderError::MissingField(_) => None,
        }
    }
}

impl std::fmt::Display for EventBuilderError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            EventBuilderError::MissingField(ref s) => write!(f, "MissingField: {}", s),
        }
    }
}

#[derive(Default, Clone)]
pub struct EventBuilder {
    pub event_type: Option<String>,
    pub attributes: Vec<(String, String)>,
    pub data: Vec<u8>,
}

impl EventBuilder {
    pub fn new() -> Self {
        EventBuilder::default()
    }

    pub fn with_event_type(mut self, event_type: String) -> EventBuilder {
        self.event_type = Some(event_type);
        self
    }

    pub fn with_attributes(mut self, attributes: Vec<(String, String)>) -> EventBuilder {
        self.attributes = attributes;
        self
    }

    pub fn with_data(mut self, data: Vec<u8>) -> EventBuilder {
        self.data = data;
        self
    }

    pub fn build(self) -> Result<Event, EventBuilderError> {
        let event_type = self.event_type.ok_or_else(|| {
            EventBuilderError::MissingField("'event_type' field is required".to_string())
        })?;

        Ok(Event {
            event_type,
            attributes: self.attributes,
            data: self.data,
        })
    }
}

#[derive(Debug)]
pub enum TransactionReceiptBuilderError {
    MissingField(String),
}

impl StdError for TransactionReceiptBuilderError {
    fn description(&self) -> &str {
        match *self {
            TransactionReceiptBuilderError::MissingField(ref msg) => msg,
        }
    }

    fn cause(&self) -> Option<&dyn StdError> {
        match *self {
            TransactionReceiptBuilderError::MissingField(_) => None,
        }
    }
}

impl std::fmt::Display for TransactionReceiptBuilderError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            TransactionReceiptBuilderError::MissingField(ref s) => write!(f, "MissingField: {}", s),
        }
    }
}

#[derive(Default, Clone)]
pub struct TransactionReceiptBuilder {
    pub state_changes: Vec<StateChange>,
    pub events: Vec<Event>,
    pub data: Vec<Vec<u8>>,
    pub transaction_id: Option<String>,
}

impl TransactionReceiptBuilder {
    pub fn new() -> Self {
        TransactionReceiptBuilder::default()
    }

    pub fn with_state_changes(
        mut self,
        state_changes: Vec<StateChange>,
    ) -> TransactionReceiptBuilder {
        self.state_changes = state_changes;
        self
    }

    pub fn with_events(mut self, events: Vec<Event>) -> TransactionReceiptBuilder {
        self.events = events;
        self
    }

    pub fn with_data(mut self, data: Vec<Vec<u8>>) -> TransactionReceiptBuilder {
        self.data = data;
        self
    }

    pub fn with_transaction_id(mut self, transaction_id: String) -> TransactionReceiptBuilder {
        self.transaction_id = Some(transaction_id);
        self
    }

    pub fn build(self) -> Result<TransactionReceipt, TransactionReceiptBuilderError> {
        let transaction_id = self.transaction_id.ok_or_else(|| {
            TransactionReceiptBuilderError::MissingField(
                "'transaction_id' field is required".to_string(),
            )
        })?;

        Ok(TransactionReceipt {
            state_changes: self.state_changes,
            events: self.events,
            data: self.data,
            transaction_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "sawtooth-compat")]
    use sawtooth_sdk;

    static ADDRESS: &str = "5b7349700e158b598043efd6d7610345a75a00b22ac14c9278db53f586179a92b72fbd";
    static BYTES1: [u8; 4] = [0x01, 0x02, 0x03, 0x04];
    static BYTES2: [u8; 4] = [0x05, 0x06, 0x07, 0x08];
    static BYTES3: [u8; 4] = [0x09, 0x0a, 0x0b, 0x0c];
    static EVENT_TYPE1: &str = "sawtooth/block-commit";
    static EVENT_TYPE2: &str = "sawtooth/state-delta";
    static ATTR1: (&str, &str) = (
        "block_id",
        "f40b90d06b4a9074af2ab09e0187223da7466be75ec0f472 \
         f2edd5f22960d76e402e6c07c90b7816374891d698310dd25d9b88dce7dbcba8219d9f7c9cae1861",
    );
    static ATTR2: (&str, &str) = ("block_num", "3");
    static ATTR3: (&str, &str) = (
        "address",
        "5b7349700e158b598043efd6d7610345a75a00b22ac14c9278db53f586179a92b72fbd",
    );
    static TRANSACTION_ID: &str =
        "24b168aaf5ea4a76a6c316924a1c26df0878908682ea5740dd70814e \
         7c400d56354dee788191be8e28393c70398906fb467fac8db6279e90e4e61619589d42bf";

    pub fn make_event_1() -> Event {
        Event {
            event_type: EVENT_TYPE1.to_string(),
            attributes: vec![
                (ATTR1.0.to_string(), ATTR1.1.to_string()),
                (ATTR2.0.to_string(), ATTR2.1.to_string()),
            ],
            data: BYTES2.to_vec(),
        }
    }

    pub fn make_event_2() -> Event {
        Event {
            event_type: EVENT_TYPE2.to_string(),
            attributes: vec![(ATTR3.0.to_string(), ATTR3.1.to_string())],
            data: BYTES3.to_vec(),
        }
    }

    #[test]
    fn state_change_fields() {
        let state_change_set = StateChange::Set {
            key: ADDRESS.to_string(),
            value: BYTES1.to_vec(),
        };
        check_state_change(state_change_set);
        let state_change_delete = StateChange::Delete {
            key: ADDRESS.to_string(),
        };
        check_state_change(state_change_delete);
    }

    fn check_state_change(state_change: StateChange) {
        match state_change {
            StateChange::Set { key, value } => {
                assert_eq!(ADDRESS, key);
                assert_eq!(BYTES1.to_vec(), value);
            }
            StateChange::Delete { key } => {
                assert_eq!(ADDRESS, key);
            }
        }
    }

    #[test]
    fn state_state_change_conversion() {
        let receipt_state_change_set = StateChange::Set {
            key: ADDRESS.to_string(),
            value: BYTES1.to_vec(),
        };
        let change: state::StateChange = receipt_state_change_set.into();
        check_state_state_change(change);

        let receipt_state_change_delete = StateChange::Delete {
            key: ADDRESS.to_string(),
        };
        let change: state::StateChange = receipt_state_change_delete.into();
        check_state_state_change(change);
    }

    fn check_state_state_change(state_change: state::StateChange) {
        match state_change {
            state::StateChange::Set { key, value } => {
                assert_eq!(ADDRESS, key);
                assert_eq!(BYTES1.to_vec(), value);
            }
            state::StateChange::Delete { key } => {
                assert_eq!(ADDRESS, key);
            }
        }
    }

    #[test]
    fn event_fields() {
        check_event(make_event_1());
    }

    fn check_event(event: Event) {
        assert_eq!(EVENT_TYPE1, event.event_type);
        assert_eq!(
            (ATTR1.0.to_string(), ATTR1.1.to_string()),
            event.attributes[0]
        );
        assert_eq!(
            (ATTR2.0.to_string(), ATTR2.1.to_string()),
            event.attributes[1]
        );
        assert_eq!(BYTES2.to_vec(), event.data);
    }

    #[test]
    fn transaction_receipt_fields() {
        let transaction_receipt = TransactionReceipt {
            state_changes: vec![
                StateChange::Set {
                    key: ADDRESS.to_string(),
                    value: BYTES1.to_vec(),
                },
                StateChange::Delete {
                    key: ADDRESS.to_string(),
                },
            ],
            events: vec![make_event_1(), make_event_2()],
            data: vec![BYTES1.to_vec(), BYTES2.to_vec(), BYTES3.to_vec()],
            transaction_id: TRANSACTION_ID.to_string(),
        };

        check_transaction_receipt(transaction_receipt)
    }

    #[test]
    // test that the transaction receipts can be converted into bytes and back correctly
    fn transaction_receipt_bytes() {
        let original = TransactionReceipt {
            state_changes: vec![
                StateChange::Set {
                    key: ADDRESS.to_string(),
                    value: BYTES1.to_vec(),
                },
                StateChange::Delete {
                    key: ADDRESS.to_string(),
                },
            ],
            events: vec![make_event_1(), make_event_2()],
            data: vec![BYTES1.to_vec(), BYTES2.to_vec(), BYTES3.to_vec()],
            transaction_id: TRANSACTION_ID.to_string(),
        };

        let receipt_bytes = original.clone().into_bytes().unwrap();
        let receipt = TransactionReceipt::from_bytes(&receipt_bytes).unwrap();

        check_transaction_receipt(receipt.clone());
        assert_eq!(original.transaction_id, receipt.transaction_id);
    }

    fn check_transaction_receipt(transaction_receipt: TransactionReceipt) {
        for state_change in transaction_receipt.state_changes {
            check_state_change(state_change)
        }
        assert_eq!(
            vec!(make_event_1(), make_event_2()),
            transaction_receipt.events
        );
        assert_eq!(
            vec!(BYTES1.to_vec(), BYTES2.to_vec(), BYTES3.to_vec()),
            transaction_receipt.data
        );
    }

    #[cfg(feature = "sawtooth-compat")]
    #[test]
    fn transaction_receipt_sawtooth10_compatibility() {
        let mut proto_transaction_receipt =
            sawtooth_sdk::messages::transaction_receipt::TransactionReceipt::new();
        let mut proto_state_change_set =
            sawtooth_sdk::messages::transaction_receipt::StateChange::new();
        proto_state_change_set.set_address(ADDRESS.to_string());
        proto_state_change_set
            .set_field_type(sawtooth_sdk::messages::transaction_receipt::StateChange_Type::SET);
        proto_state_change_set.set_value(BYTES1.to_vec());
        let mut proto_state_change_delete =
            sawtooth_sdk::messages::transaction_receipt::StateChange::new();
        proto_state_change_delete.set_address(ADDRESS.to_string());
        proto_state_change_delete
            .set_field_type(sawtooth_sdk::messages::transaction_receipt::StateChange_Type::DELETE);
        proto_transaction_receipt.set_state_changes(protobuf::RepeatedField::from_vec(vec![
            proto_state_change_set,
            proto_state_change_delete,
        ]));
        let mut proto_event = sawtooth_sdk::messages::events::Event::new();
        proto_event.set_event_type(EVENT_TYPE2.to_string());
        let mut event_attr = sawtooth_sdk::messages::events::Event_Attribute::new();
        event_attr.set_key(ATTR3.0.to_string());
        event_attr.set_value(ATTR3.1.to_string());
        proto_event.set_attributes(protobuf::RepeatedField::from_vec(vec![event_attr]));
        proto_event.set_data(BYTES3.to_vec());
        proto_transaction_receipt.set_events(protobuf::RepeatedField::from_vec(vec![proto_event]));
        proto_transaction_receipt
            .set_data(protobuf::RepeatedField::from_vec(vec![BYTES2.to_vec()]));
        proto_transaction_receipt.set_transaction_id(TRANSACTION_ID.to_string());

        let transaction_receipt_bytes =
            protobuf::Message::write_to_bytes(&proto_transaction_receipt).unwrap();

        let proto: protos::transaction_receipt::TransactionReceipt =
            protobuf::parse_from_bytes(&transaction_receipt_bytes).unwrap();

        let transaction_receipt: TransactionReceipt = proto.into_native().unwrap();
        for state_change in transaction_receipt.state_changes {
            check_state_change(state_change)
        }
        assert_eq!(vec!(make_event_2()), transaction_receipt.events);
        assert_eq!(vec!(BYTES2.to_vec(),), transaction_receipt.data);
    }

    #[test]
    fn transaction_receipt_builder_chain() {
        let transaction_receipt = TransactionReceiptBuilder::new()
            .with_state_changes(vec![
                StateChange::Set {
                    key: ADDRESS.to_string(),
                    value: BYTES1.to_vec(),
                },
                StateChange::Delete {
                    key: ADDRESS.to_string(),
                },
            ])
            .with_events(vec![make_event_1(), make_event_2()])
            .with_data(vec![BYTES1.to_vec(), BYTES2.to_vec(), BYTES3.to_vec()])
            .with_transaction_id(TRANSACTION_ID.to_string())
            .build()
            .unwrap();

        check_transaction_receipt(transaction_receipt)
    }

    #[test]
    fn transaction_receipt_builder_separate() {
        let mut transaction_receipt_builder = TransactionReceiptBuilder::new();
        transaction_receipt_builder = transaction_receipt_builder.with_state_changes(vec![
            StateChange::Set {
                key: ADDRESS.to_string(),
                value: BYTES1.to_vec(),
            },
            StateChange::Delete {
                key: ADDRESS.to_string(),
            },
        ]);
        transaction_receipt_builder =
            transaction_receipt_builder.with_events(vec![make_event_1(), make_event_2()]);
        transaction_receipt_builder = transaction_receipt_builder.with_data(vec![
            BYTES1.to_vec(),
            BYTES2.to_vec(),
            BYTES3.to_vec(),
        ]);
        transaction_receipt_builder =
            transaction_receipt_builder.with_transaction_id(TRANSACTION_ID.to_string());
        let transaction_receipt = transaction_receipt_builder.build().unwrap();

        check_transaction_receipt(transaction_receipt)
    }

    #[test]
    fn event_builder_chain() {
        let event = EventBuilder::new()
            .with_event_type(EVENT_TYPE1.to_string())
            .with_attributes(vec![
                (ATTR1.0.to_string(), ATTR1.1.to_string()),
                (ATTR2.0.to_string(), ATTR2.1.to_string()),
            ])
            .with_data(BYTES2.to_vec())
            .build()
            .unwrap();

        check_event(event);
    }

    #[test]
    // test that the transaction receipts can be converted into bytes and back correctly
    fn event_builder_chain_bytes() {
        let original = EventBuilder::new()
            .with_event_type(EVENT_TYPE1.to_string())
            .with_attributes(vec![
                (ATTR1.0.to_string(), ATTR1.1.to_string()),
                (ATTR2.0.to_string(), ATTR2.1.to_string()),
            ])
            .with_data(BYTES2.to_vec())
            .build()
            .unwrap();

        let event_bytes = original.clone().into_bytes().unwrap();
        let event = Event::from_bytes(&event_bytes).unwrap();
        check_event(event);
    }

    #[test]
    fn event_builder_separate() {
        let mut event_builder = EventBuilder::new();
        event_builder = event_builder.with_event_type(EVENT_TYPE1.to_string());
        event_builder = event_builder.with_attributes(vec![
            (ATTR1.0.to_string(), ATTR1.1.to_string()),
            (ATTR2.0.to_string(), ATTR2.1.to_string()),
        ]);
        event_builder = event_builder.with_data(BYTES2.to_vec());
        let event = event_builder.build().unwrap();

        check_event(event);
    }
}

#[cfg(all(feature = "nightly", test))]
mod benchmarks {
    extern crate test;
    use super::tests::{make_event_1, make_event_2};
    use super::*;
    use test::Bencher;

    static ADDRESS: &str = "5b7349700e158b598043efd6d7610345a75a00b22ac14c9278db53f586179a92b72fbd";
    static BYTES1: [u8; 4] = [0x01, 0x02, 0x03, 0x04];
    static BYTES2: [u8; 4] = [0x05, 0x06, 0x07, 0x08];
    static BYTES3: [u8; 4] = [0x09, 0x0a, 0x0b, 0x0c];
    static EVENT_TYPE1: &str = "sawtooth/block-commit";
    static ATTR1: (&str, &str) = (
        "block_id",
        "f40b90d06b4a9074af2ab09e0187223da7466be75ec0f472 \
         f2edd5f22960d76e402e6c07c90b7816374891d698310dd25d9b88dce7dbcba8219d9f7c9cae1861",
    );
    static ATTR2: (&str, &str) = ("block_num", "3");
    static ATTR3: (&str, &str) = (
        "address",
        "5b7349700e158b598043efd6d7610345a75a00b22ac14c9278db53f586179a92b72fbd",
    );
    static TRANSACTION_ID: &str =
        "24b168aaf5ea4a76a6c316924a1c26df0878908682ea5740dd70814e \
         7c400d56354dee788191be8e28393c70398906fb467fac8db6279e90e4e61619589d42bf";

    #[bench]
    fn bench_txn_receipt_creation(b: &mut Bencher) {
        b.iter(|| TransactionReceipt {
            state_changes: vec![
                StateChange::Set {
                    key: ADDRESS.to_string(),
                    value: BYTES1.to_vec(),
                },
                StateChange::Delete {
                    key: ADDRESS.to_string(),
                },
            ],
            events: vec![make_event_1(), make_event_2()],
            data: vec![BYTES1.to_vec(), BYTES2.to_vec(), BYTES3.to_vec()],
            transaction_id: TRANSACTION_ID.to_string(),
        });
    }

    #[bench]
    fn bench_txn_receipt_builder(b: &mut Bencher) {
        let transaction_receipt = TransactionReceiptBuilder::new()
            .with_state_changes(vec![
                StateChange::Set {
                    key: ADDRESS.to_string(),
                    value: BYTES1.to_vec(),
                },
                StateChange::Delete {
                    key: ADDRESS.to_string(),
                },
            ])
            .with_events(vec![make_event_1(), make_event_2()])
            .with_data(vec![BYTES1.to_vec(), BYTES2.to_vec(), BYTES3.to_vec()])
            .with_transaction_id(TRANSACTION_ID.to_string());

        b.iter(|| transaction_receipt.clone().build());
    }

    #[bench]
    fn bench_txn_receipt_into_native(b: &mut Bencher) {
        let mut proto_transaction_receipt = protos::transaction_receipt::TransactionReceipt::new();

        let mut proto_state_change = protos::transaction_receipt::StateChange::new();
        proto_state_change.set_address(ADDRESS.to_string());
        proto_state_change.set_value(BYTES1.to_vec());
        proto_state_change.set_field_type(protos::transaction_receipt::StateChange_Type::SET);
        proto_transaction_receipt
            .set_state_changes(protobuf::RepeatedField::from_vec(vec![proto_state_change]));

        let mut proto_event = protos::events::Event::new();
        proto_event.set_event_type(EVENT_TYPE1.to_string());
        let mut proto_event_attribute = protos::events::Event_Attribute::new();
        proto_event_attribute.set_key(ATTR3.0.to_string());
        proto_event_attribute.set_value(ATTR3.1.to_string());
        proto_event.set_attributes(protobuf::RepeatedField::from_vec(vec![
            proto_event_attribute,
        ]));
        proto_event.set_data(BYTES3.to_vec());

        proto_transaction_receipt.set_events(protobuf::RepeatedField::from_vec(vec![proto_event]));
        proto_transaction_receipt
            .set_data(protobuf::RepeatedField::from_vec(vec![BYTES2.to_vec()]));
        proto_transaction_receipt.set_transaction_id(TRANSACTION_ID.to_string());

        b.iter(|| proto_transaction_receipt.clone().into_native());
    }

    #[bench]
    fn bench_txn_receipt_into_proto(b: &mut Bencher) {
        let transaction_receipt = TransactionReceipt {
            state_changes: vec![
                StateChange::Set {
                    key: ADDRESS.to_string(),
                    value: BYTES1.to_vec(),
                },
                StateChange::Delete {
                    key: ADDRESS.to_string(),
                },
            ],
            events: vec![make_event_1(), make_event_2()],
            data: vec![BYTES1.to_vec(), BYTES2.to_vec(), BYTES3.to_vec()],
            transaction_id: TRANSACTION_ID.to_string(),
        };

        b.iter(|| transaction_receipt.clone().into_proto());
    }

    #[bench]
    fn bench_state_change_into_native(b: &mut Bencher) {
        let mut proto_state_change = protos::transaction_receipt::StateChange::new();
        proto_state_change.set_address(ADDRESS.to_string());
        proto_state_change.set_value(BYTES1.to_vec());
        proto_state_change.set_field_type(protos::transaction_receipt::StateChange_Type::SET);
        b.iter(|| proto_state_change.clone().into_native());
    }

    #[bench]
    fn bench_state_change_into_proto(b: &mut Bencher) {
        let state_change_set = StateChange::Set {
            key: ADDRESS.to_string(),
            value: BYTES1.to_vec(),
        };
        b.iter(|| state_change_set.clone().into_proto());
    }

    #[bench]
    fn bench_event_creation(b: &mut Bencher) {
        b.iter(|| Event {
            event_type: EVENT_TYPE1.to_string(),
            attributes: vec![
                (ATTR1.0.to_string(), ATTR1.1.to_string()),
                (ATTR2.0.to_string(), ATTR2.1.to_string()),
            ],
            data: BYTES2.to_vec(),
        });
    }

    #[bench]
    fn bench_event_builder(b: &mut Bencher) {
        let event = EventBuilder::new()
            .with_event_type(EVENT_TYPE1.to_string())
            .with_attributes(vec![
                (ATTR1.0.to_string(), ATTR1.1.to_string()),
                (ATTR2.0.to_string(), ATTR2.1.to_string()),
            ])
            .with_data(BYTES2.to_vec());
        b.iter(|| event.clone().build());
    }

    #[bench]
    fn bench_event_into_proto(b: &mut Bencher) {
        let event = Event {
            event_type: EVENT_TYPE1.to_string(),
            attributes: vec![
                (ATTR1.0.to_string(), ATTR1.1.to_string()),
                (ATTR2.0.to_string(), ATTR2.1.to_string()),
            ],
            data: BYTES2.to_vec(),
        };
        b.iter(|| event.clone().into_proto());
    }

    #[bench]
    fn bench_event_into_native(b: &mut Bencher) {
        let mut proto_event = protos::events::Event::new();
        proto_event.set_event_type(EVENT_TYPE1.to_string());
        let mut proto_event_attribute = protos::events::Event_Attribute::new();
        proto_event_attribute.set_key(ATTR3.0.to_string());
        proto_event_attribute.set_value(ATTR3.1.to_string());
        proto_event.set_attributes(protobuf::RepeatedField::from_vec(vec![
            proto_event_attribute,
        ]));
        proto_event.set_data(BYTES3.to_vec());
        b.iter(|| proto_event.clone().into_native());
    }
}
