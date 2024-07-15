// @generated automatically by Diesel CLI.

diesel::table! {
    covers (id) {
        id -> Unsigned<Integer>,
        last_try -> Datetime,
        provider -> Nullable<Unsigned<Tinyint>>,
    }
}
