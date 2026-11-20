# Catalog of the doc examples and `tests/icu_trimmed.rs`.

hello = Hello
greet = Hello { $name }
unread = { $count ->
    [one] { $count } unread message
   *[other] { $count } unread messages
}
