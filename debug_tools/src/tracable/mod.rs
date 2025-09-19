// use core::fmt;
// use std::{cell::RefCell, fmt::Debug};

// use serde::ser::SerializeStruct;
// use serde_json::json;
// use utils::system;
// use uuid::Uuid;
// #[derive(Clone)]
// pub struct TraceRecord {
//     ts: u64,
//     name: String,
//     attrs: Vec<(String, String)>,
// }

// impl TraceRecord {
//     pub fn new(name: &str) -> Self {
//         TraceRecord {
//             ts: system::time::get_timestamp_ms().unwrap(),
//             name: name.to_string(),
//             attrs: vec![],
//         }
//     }

//     pub fn set_attr(&mut self, key: &str, value: &str) -> &mut Self {
//         self.attrs.push((key.to_string(), value.to_string()));
//         self
//     }
// }

// impl serde::Serialize for TraceRecord {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: serde::Serializer,
//     {
//         let mut s = serializer.serialize_struct("trace_record", 3)?;
//         s.serialize_field("name", &self.name)?;
//         s.serialize_field("ts", &self.ts)?;
//         s.serialize_field("attrs", &self.attrs)?;
//         s.end()
//     }
// }

// pub struct TraceRecordBuilder<'a> {
//     record: TraceRecord,
//     tracable: &'a dyn Tracable,
// }

// impl<'a> TraceRecordBuilder<'a> {
//     pub fn new(name: &str, tracable: &'a dyn Tracable) -> TraceRecordBuilder<'a> {
//         TraceRecordBuilder {
//             record: TraceRecord::new(name),
//             tracable,
//         }
//     }

//     pub fn attr(mut self, key: &str, value: &str) -> Self {
//         self.record.set_attr(key, value);
//         self
//     }

//     pub fn trace_complete(self) {
//         self.tracable.trace_complete(self.record);
//     }

//     pub(crate) fn null(tracable: &'a NullTracable) -> TraceRecordBuilder<'a> {
//         TraceRecordBuilder {
//             record: TraceRecord {
//                 ts: 0,
//                 name: String::new(),
//                 attrs: vec![],
//             },
//             tracable,
//         }
//     }
// }

// pub trait Tracable: Debug + Send {
//     fn trace_id(&self) -> String;
//     fn trace_name(&self) -> String;
//     fn set_trace_name(&self, name: &str);
//     fn trace(&self, name: &str) -> TraceRecordBuilder<'_>;
//     fn trace_complete(&self, record: TraceRecord);
//     fn dump_trace(&self) -> serde_json::Value;
//     fn derive_tracable(&self, other: &dyn Tracable);
//     fn tracable_type(&self) -> TracableType;
//     fn clear(&self);
// }

// #[derive(Debug, Clone, serde::Serialize)]
// pub enum TracableType {
//     Null,
//     Simple,
// }

// pub struct TracableFactory;

// impl TracableFactory {
//     pub fn create_null_tracable() -> NullTracable {
//         NullTracable
//     }

//     pub fn create_null_tracable_wrapper() -> TracableWrapper {
//         TracableWrapper::new(Box::new(Self::create_null_tracable()))
//     }

//     pub fn create_simple_tracable(id: &str, name: &str) -> SimpleTracable {
//         SimpleTracable::new(id, name)
//     }

//     pub fn create_tracable_wrapper(name: &str, tracable_type: TracableType) -> TracableWrapper {
//         TracableWrapper::new(Self::create_tracable(name, tracable_type))
//     }

//     pub fn create_tracable(name: &str, tracable_type: TracableType) -> Box<dyn Tracable> {
//         match tracable_type {
//             TracableType::Null => Box::new(Self::create_null_tracable()),
//             TracableType::Simple => Box::new(Self::create_simple_tracable(
//                 Uuid::now_v7().to_string().as_str(),
//                 name,
//             )),
//         }
//     }
// }

// pub struct TracableWrapper {
//     tracable: Box<dyn Tracable>,
// }

// impl Clone for TracableWrapper {
//     fn clone(&self) -> Self {
//         let new_tracable = TracableFactory::create_tracable(
//             self.tracable.trace_name().as_str(),
//             self.tracable.tracable_type(),
//         );
//         new_tracable.derive_tracable(self.tracable.as_ref());
//         Self {
//             tracable: new_tracable,
//         }
//     }
// }

// impl TracableWrapper {
//     pub fn new(tracable: Box<dyn Tracable>) -> Self {
//         Self { tracable }
//     }
// }

// impl fmt::Debug for TracableWrapper {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         write!(
//             f,
//             "\nTracableWrapper[{} - {} - {:?}]\n{}",
//             self.tracable.trace_id(),
//             self.tracable.trace_name(),
//             self.tracable.tracable_type(),
//             self.tracable.dump_trace(),
//         )
//     }
// }

// impl Tracable for TracableWrapper {
//     fn tracable_type(&self) -> TracableType {
//         self.tracable.tracable_type()
//     }
//     fn trace_id(&self) -> String {
//         self.tracable.trace_id()
//     }

//     fn trace_name(&self) -> String {
//         self.tracable.trace_name()
//     }
//     fn set_trace_name(&self, name: &str) {
//         self.tracable.set_trace_name(name);
//     }

//     fn trace(&self, name: &str) -> TraceRecordBuilder<'_> {
//         self.tracable.trace(name)
//     }
//     fn trace_complete(&self, record: TraceRecord) {
//         self.tracable.trace_complete(record);
//     }
//     fn dump_trace(&self) -> serde_json::Value {
//         self.tracable.dump_trace()
//     }
//     fn derive_tracable(&self, other: &dyn Tracable) {
//         self.tracable.derive_tracable(other);
//     }
//     fn clear(&self) {
//         self.tracable.clear();
//     }
// }

// pub struct NullTracable;

// impl fmt::Debug for NullTracable {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         write!(f, "NullTracable")
//     }
// }

// impl Tracable for NullTracable {
//     fn tracable_type(&self) -> TracableType {
//         TracableType::Null
//     }
//     fn trace_id(&self) -> String {
//         "null".to_string()
//     }

//     fn trace_name(&self) -> String {
//         "null".to_string()
//     }

//     fn set_trace_name(&self, _name: &str) {}

//     fn trace(&self, _name: &str) -> TraceRecordBuilder<'_> {
//         TraceRecordBuilder::null(self)
//     }
//     fn trace_complete(&self, _record: TraceRecord) {}
//     fn dump_trace(&self) -> serde_json::Value {
//         json!("null tracable")
//     }
//     fn derive_tracable(&self, _other: &dyn Tracable) {}
//     fn clear(&self) {}
// }

// #[derive(Clone)]
// pub struct SimpleTracable {
//     trace_id: RefCell<String>,
//     trace_name: RefCell<String>,
//     trace_info: RefCell<Vec<TraceRecord>>,
//     from: RefCell<Vec<serde_json::Value>>,
// }

// impl SimpleTracable {
//     pub fn new(id: &str, name: &str) -> Self {
//         Self {
//             trace_id: RefCell::new(id.to_string()),
//             trace_name: RefCell::new(name.to_string()),
//             trace_info: RefCell::new(vec![]),
//             from: RefCell::new(vec![]),
//         }
//     }

//     pub fn with_id(id: &str) -> Self {
//         Self::new(id, "")
//     }

//     pub fn with_name(name: &str) -> Self {
//         Self::new("", name)
//     }
// }

// impl fmt::Debug for SimpleTracable {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         write!(f, "{}", self.dump_trace())
//     }
// }

// impl Tracable for SimpleTracable {
//     fn tracable_type(&self) -> TracableType {
//         TracableType::Simple
//     }
//     fn trace_id(&self) -> String {
//         self.trace_id.borrow().clone()
//     }

//     fn trace_name(&self) -> String {
//         self.trace_name.borrow().clone()
//     }

//     fn set_trace_name(&self, name: &str) {
//         *self.trace_name.borrow_mut() = name.to_string();
//     }

//     fn trace(&self, name: &str) -> TraceRecordBuilder<'_> {
//         TraceRecordBuilder::new(name, self)
//     }

//     fn trace_complete(&self, record: TraceRecord) {
//         self.trace_info.borrow_mut().push(record);
//     }

//     fn dump_trace(&self) -> serde_json::Value {
//         json!({
//             "id": self.trace_id(),
//             "name": self.trace_name(),
//             "trace_type": self.tracable_type(),
//             "attrs": self.trace_info,
//             "from": self.from,
//         })
//     }

//     fn derive_tracable(&self, other: &dyn Tracable) {
//         let other_trace = other.dump_trace();
//         self.from.borrow_mut().push(other_trace);
//     }
//     fn clear(&self) {
//         self.trace_info.borrow_mut().clear();
//         self.from.borrow_mut().clear();
//     }
// }

// #[macro_export]
// macro_rules! function_name {
//     () => {{
//         fn f() {}
//         fn type_name_of<T>(_: T) -> &'static str {
//             std::any::type_name::<T>()
//         }
//         let name = type_name_of(f);
//         name.strip_suffix("::f").unwrap()
//     }};
// }

// #[macro_export]
// macro_rules! media_trace {
//     ($tracable:expr, $name:expr, $( $key:expr => $value:expr ),* $(,)?,) => {
//         {
//             use $crate::tracable::Tracable;
//             let mut builder = $tracable.trace($name);
//             $(
//                 builder = builder.attr($key, &format!("{}", $value));
//             )*
//             builder = builder
//                 .attr("method", $crate::function_name!());
//             builder.trace_complete();
//         }
//     };
//     (media: $media:expr, $name:expr, $( $key:expr => $value:expr ),* $(,)?,) => {
//         $crate::media_trace!(
//             $media.tracable,
//             $name,
//             "data_type" => &format!("{}", std::any::type_name_of_val(&$media)),
//             $($key => $value),*,
//         );
//     };
//     (tracable: $tracable:expr, $name:expr, $( $key:expr => $value:expr ),* $(,)?,) => {
//         $crate::media_trace!($tracable, $name, $($key => $value),*,);
//     };
// }

// #[macro_export]
// macro_rules! media_trace_queue {
//     (tracable: $tracable:expr, kind: $kind:expr, queue_size: $queue_size:expr, queue_name: $queue_name:expr, $( $key:expr => $value:expr ),* $(,)?,) => {
//         $crate::media_trace!(tracable: $tracable, $kind,
//             "queue_name" => $queue_name,
//             "queue_size" => $queue_size,
//             $($key => $value),*,
//         );
//     };
//     (media: $media:expr, kind: $kind:expr, queue_size: $queue_size:expr, queue_name: $queue_name:expr, $( $key:expr => $value:expr ),* $(,)?,) => {
//         $crate::media_trace!(media: $media, $kind,
//             "queue_name" => $queue_name,
//             "queue_size" => $queue_size,
//             $($key => $value),*,
//         );
//     };
//     ($tracable:expr, $kind:expr, $queue_size:expr, $queue_name:expr, $( $key:expr => $value:expr ),* $(,)?,) => {
//         $crate::media_trace_queue!(tracable: $tracable, kind: $kind, queue_size: $queue_size, queue_name: $queue_name, $($key => $value),*,);
//     }
// }

// #[macro_export]
// macro_rules! media_trace_enqueue {
//     (tracable: $tracable:expr, queue_size: $queue_size:expr, queue_name: $queue_name:expr, $( $key:expr => $value:expr ),* $(,)?,) => {
//         $crate::media_trace_queue!(tracable: $tracable, kind: "enqueue", queue_size: $queue_size, queue_name: $queue_name, $($key => $value),*,);
//     };
//     (media: $media:expr, queue_size: $queue_size:expr, queue_name: $queue_name:expr, $( $key:expr => $value:expr ),* $(,)?,) => {
//         $crate::media_trace_queue!(media: $media, kind: "enqueue", queue_size: $queue_size, queue_name: $queue_name, $($key => $value),*,);
//     }
// }

// #[macro_export]
// macro_rules! media_trace_dequeue {
//     (tracable: $tracable:expr, queue_size: $queue_size:expr, queue_name: $queue_name:expr, $( $key:expr => $value:expr ),* $(,)?,) => {
//         $crate::media_trace_queue!(tracable: $tracable, kind: "dequeue", queue_size: $queue_size, queue_name: $queue_name, $($key => $value),*,);
//     };
//     (media: $media:expr, queue_size: $queue_size:expr, queue_name: $queue_name:expr, $( $key:expr => $value:expr ),* $(,)?,) => {
//         $crate::media_trace_queue!(media: $media, kind: "dequeue", queue_size: $queue_size, queue_name: $queue_name, $($key => $value),*,);
//     }
// }

// #[macro_export]
// macro_rules! media_trace_drop {
//     (tracable: $tracable:expr, reason: $reason:expr, $( $key:expr => $value:expr ),* $(,)?,) => {
//         $crate::media_trace!(tracable: $tracable, "drop",
//             "reason" => $reason,
//             $($key => $value),*,
//         );
//     };
//     (media: $media:expr, reason: $reason:expr, $( $key:expr => $value:expr ),* $(,)?,) => {
//         $crate::media_trace!(media: $media, "drop",
//             "reason" => $reason,
//             $($key => $value),*,
//         );
//     }
// }

// #[macro_export]
// macro_rules! media_trace_receive {
//     (tracable: $tracable:expr, from: $from:expr, session: $session:expr, $( $key:expr => $value:expr ),* $(,)?,) => {
//         $crate::media_trace!(tracable: $tracable, "receive",
//             "from" => $from,
//             "session" => $session,
//             $($key => $value),*,
//         );
//     };
//     (media: $media:expr, from: $from:expr, session: $session:expr, $( $key:expr => $value:expr ),* $(,)?,) => {
//         $crate::media_trace!(media: $media, "receive",
//             "from" => $from,
//             "session" => $session,
//             $($key => $value),*,
//         );
//     }
// }

// #[macro_export]
// macro_rules! media_trace_send {
//     (tracable: $tracable:expr, to: $to:expr, session: $session:expr, $( $key:expr => $value:expr ),* $(,)?,) => {
//         $crate::media_trace!(tracable: $tracable, "send",
//             "to" => $to,
//             "session" => $session,
//             $($key => $value),*,
//         );
//     };
//     (media: $media:expr, to: $to:expr, session: $session:expr, $( $key:expr => $value:expr ),* $(,)?,) => {
//         $crate::media_trace!(media: $media, "send",
//             "to" => $to,
//             "session" => $session,
//             $($key => $value),*,
//         );
//     }
// }

// #[macro_export]
// macro_rules! media_trace_merge {
//     (from: $from_tracable:expr, to: $to_traceable:expr, $( $key:expr => $value:expr ),* $(,)?,) => {
//         $to_traceable.derive_tracable(&$from_tracable);
//         $crate::media_trace!(tracable: $to_traceable, "merge", "merge_from" => $from_tracable.trace_id(), $($key => $value),*,);
//     };
//     (from_media: $from_media:expr, to_media: $to_media:expr, $( $key:expr => $value:expr ),* $(,)?,) => {
//        $to_media.tracable.derive_tracable(&$from_media.tracable);
//        $crate::media_trace_merge!(from: $from_media.tracable, to: $to_media.tracable,
//            "from_type" => std::any::type_name_of_val(&$from_media),
//            "to_type" => std::any::type_name_of_val(&$to_media),
//            $($key => $value),*,
//         );
//     };
//     (from: $from_tracable:expr, to_media: $to_media:expr, $( $key:expr => $value:expr ),* $(,)?,) => {
//        $to_media.tracable.derive_tracable(&$from_tracable);
//        $crate::media_trace_merge!(from: $from_tracable, to: $to_media.tracable,
//            "to_type" => std::any::type_name_of_val(&$to_media),
//            $($key => $value),*,
//         );
//     };
//     (from_media: $from_media:expr, to: $to_traceable:expr, $( $key:expr => $value:expr ),* $(,)?,) => {
//        $to_tracable.derive_tracable(&$from_media.tracable);
//        $crate::media_trace_merge!(from: $from_media.tracable, to: $to_tracable,
//            "from_type" => std::any::type_name_of_val(&$from_media),
//            $($key => $value),*,
//         );
//     }
// }
// #[macro_export]
// macro_rules! media_trace_split {
//     (from: $from_tracable:expr, to: $to_traceable:expr, $( $key:expr => $value:expr ),* $(,)?,) => {
//         $to_traceable.derive_tracable(&$from_tracable);
//         $crate::media_trace!(tracable: $to_traceable, "split", "split_from" => $from_tracable.trace_id(), $($key => $value),*,);
//     };
//     (from_media: $from_meida:expr, to_media: $to_media:expr, $( $key:expr => $value:expr ),+ $(,)?,) => {
//        $to_media.tracable.derive_tracable(&$from_media.tracable);
//        $crate::media_trace_split!(from: $from_media.tracable, to: $to_media.tracable,
//            "from_type" => std::any::type_name_of_val(&$from_media),
//            "to_type" => std::any::type_name_of_val(&$to_media),
//            $($key => $value),*,
//         );
//     };
//     (from: $from_tracable:expr, to_media: $to_media:expr, $( $key:expr => $value:expr ),* $(,)?,) => {
//        $to_media.tracable.derive_tracable(&$from_tracable);
//        $crate::media_trace_split!(from: $from_tracable, to: $to_media.tracable,
//            "to_type" => std::any::type_name_of_val(&$to_media),
//            $($key => $value),*,
//         );
//     };
//     (from_media: $from_media:expr, to: $to_traceable:expr, $( $key:expr => $value:expr ),* $(,)?,) => {
//        $to_tracable.derive_tracable(&$from_media.tracable);
//        $crate::media_trace_split!(from: $from_media.tracable, to: $to_tracable,
//            "from_type" => std::any::type_name_of_val(&$from_media),
//            $($key => $value),*,
//         );
//     }
// }

// #[macro_export]
// macro_rules! media_trace_convert {
//     (from: $from_tracable:expr, to: $to_traceable:expr, $( $key:expr => $value:expr ),* $(,)?,) => {
//         {
//             use $crate::tracable::Tracable;
//             $to_traceable.derive_tracable($from_tracable);
//             $crate::media_trace!(tracable: $to_traceable, "convert", "convert_from" => $from_tracable.trace_id(), $($key => $value),*,);
//         }
//     };
//     (from_media: $from_media:expr, to_media: $to_media:expr, $( $key:expr => $value:expr ),* $(,)?,) => {
//        $to_media.tracable.derive_tracable(&$from_media.tracable);
//        $crate::media_trace_convert!(from: &$from_media.tracable, to: $to_media.tracable,
//            "from_type" => std::any::type_name_of_val(&$from_media),
//            "to_type" => std::any::type_name_of_val(&$to_media),
//            $($key => $value),*,
//         );
//     };
//     (from: $from_tracable:expr, to_media: $to_media:expr, $( $key:expr => $value:expr ),* $(,)?,) => {
//         {
//             use $crate::tracable::Tracable;
//             $to_media.tracable.derive_tracable($from_tracable);
//             $crate::media_trace_convert!(from: $from_tracable, to: $to_media.tracable,
//                 "to_type" => std::any::type_name_of_val(&$to_media),
//                 $($key => $value),*,
//             );
//         }
//     };
//     (from_media: $from_media:expr, to: $to_traceable:expr, $( $key:expr => $value:expr ),* $(,)?,) => {
//        $to_tracable.derive_tracable(&$from_media.tracable);
//        $crate::media_trace_convert!(from: $from_media.tracable, to: $to_tracable,
//            "from_type" => std::any::type_name_of_val(&$from_media),
//            $($key => $value),*,
//         );
//     }
// }
