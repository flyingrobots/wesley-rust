//! Rust names for foreign names (GraphQL fields, JSON members, wire values).

/// A PascalCase type name.
#[must_use]
pub fn type_name(name: &str) -> String {
    let _ = name;
    unimplemented!()
}

/// A snake_case field name.
#[must_use]
pub fn field_name(name: &str) -> String {
    let _ = name;
    unimplemented!()
}

/// A PascalCase enum variant name for a wire value.
#[must_use]
pub fn variant_name(wire: &str) -> String {
    let _ = wire;
    unimplemented!()
}

/// A plain snake_case form of `name`, with no keyword handling; for building
/// a longer identifier such as a function name stem.
#[must_use]
pub fn snake_case(name: &str) -> String {
    let _ = name;
    unimplemented!()
}
