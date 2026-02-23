use rootcause::{
    Report,
    markers::{Mutable, ObjectMarkerFor},
};

#[derive(Debug)]
struct TraceLocation {
    message: Option<String>,
    file: &'static str,
    line: u32,
}

impl std::fmt::Display for TraceLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.file, self.line)?;
        match self.message.as_ref() {
            Some(msg) => write!(f, " @ {}", msg),
            None => Ok(()),
        }
    }
}

impl TraceLocation {
    #[track_caller]
    fn new_with_message(message: String) -> Self {
        let location = std::panic::Location::caller();
        Self {
            message: Some(message),
            file: location.file(),
            line: location.line(),
        }
    }

    #[track_caller]
    fn new() -> Self {
        let location = std::panic::Location::caller();
        Self {
            message: None,
            file: location.file(),
            line: location.line(),
        }
    }
}

pub trait LocationExt {
    #[track_caller]
    fn trace_with_message(self, header: impl ToString) -> Self;
    #[track_caller]
    fn trace(self) -> Self;
    #[track_caller]
    fn trace_with<F>(self, header: F) -> Self
    where
        F: FnOnce() -> String;
}

impl<C: ?Sized, T> LocationExt for Report<C, Mutable, T>
where
    TraceLocation: ObjectMarkerFor<T>,
{
    fn trace_with_message(self, header: impl ToString) -> Self {
        self.attach(TraceLocation::new_with_message(header.to_string()))
    }
    fn trace(self) -> Self {
        self.attach(TraceLocation::new())
    }
    fn trace_with<F>(self, header: F) -> Self
    where
        F: FnOnce() -> String,
    {
        self.trace_with_message(header())
    }
}

impl<V, C: ?Sized, T> LocationExt for Result<V, Report<C, Mutable, T>>
where
    TraceLocation: ObjectMarkerFor<T>,
{
    fn trace_with_message(self, header: impl ToString) -> Self {
        match self {
            Ok(value) => Ok(value),
            Err(report) => Err(report.trace_with_message(header)),
        }
    }

    fn trace(self) -> Self {
        match self {
            Ok(value) => Ok(value),
            Err(report) => Err(report.trace()),
        }
    }

    fn trace_with<F>(self, header: F) -> Self
    where
        F: FnOnce() -> String,
    {
        match self {
            Ok(value) => Ok(value),
            Err(report) => Err(report.trace_with_message(header())),
        }
    }
}

/// Semantic context attachment for operations (no location capture).
///
/// Use these to add meaningful context about what operation was being performed
/// or what resource was being accessed when an error occurred.
#[derive(Debug)]
pub struct OperationContext(pub String);

impl std::fmt::Display for OperationContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "during: {}", self.0)
    }
}

#[derive(Debug)]
pub struct ResourceContext {
    pub name: String,
    pub id: String,
}

impl std::fmt::Display for ResourceContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.name, self.id)
    }
}

/// Extension trait for adding semantic context to errors.
///
/// Unlike `LocationExt` which captures call-site location, these methods
/// add meaningful operation/resource context without location tracking.
pub trait ContextExt {
    /// Attach context describing the operation being performed.
    ///
    /// # Example
    /// ```ignore
    /// socket.connect(addr).operation("connecting to TURN server")?;
    /// ```
    fn operation(self, op: impl ToString) -> Self;

    /// Attach context describing the resource being accessed.
    ///
    /// # Example
    /// ```ignore
    /// allocation.refresh().resource("allocation", transaction_id)?;
    /// ```
    fn resource(self, name: &str, id: impl ToString) -> Self;
}

impl<C: ?Sized, T> ContextExt for Report<C, Mutable, T>
where
    OperationContext: ObjectMarkerFor<T>,
    ResourceContext: ObjectMarkerFor<T>,
{
    fn operation(self, op: impl ToString) -> Self {
        self.attach(OperationContext(op.to_string()))
    }

    fn resource(self, name: &str, id: impl ToString) -> Self {
        self.attach(ResourceContext {
            name: name.to_string(),
            id: id.to_string(),
        })
    }
}

impl<V, C: ?Sized, T> ContextExt for Result<V, Report<C, Mutable, T>>
where
    OperationContext: ObjectMarkerFor<T>,
    ResourceContext: ObjectMarkerFor<T>,
{
    fn operation(self, op: impl ToString) -> Self {
        match self {
            Ok(value) => Ok(value),
            Err(report) => Err(report.operation(op)),
        }
    }

    fn resource(self, name: &str, id: impl ToString) -> Self {
        match self {
            Ok(value) => Ok(value),
            Err(report) => Err(report.resource(name, id)),
        }
    }
}
