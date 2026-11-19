# egui_mineral_material component strings (MD3 / Flutter MaterialLocalizations wording).
# Translations: same ids in `<locale>.ftl`, registered with `egui_mineral::i18n::add_resource`
# or provided by the app localizer.

mineral-ok = OK
mineral-cancel = Cancel
mineral-close = Close
mineral-window-close = Close window
mineral-back = Back
mineral-menu = Open navigation menu
mineral-menu-close = Close navigation menu
mineral-dismiss = Dismiss
mineral-search = Search
mineral-suggestions = Suggestions
mineral-search-clear = Clear text
mineral-navigation = Navigation
mineral-menu-strip = Menu bar
mineral-split-button-menu = More options
mineral-progress = Progress
mineral-percent = { NUMBER($value, maximumFractionDigits: 0) }%
mineral-range-value = { $start } - { $end }
mineral-range-start = { $label } start
mineral-range-end = { $label } end
mineral-badge-overflow = { $max }+
mineral-chip-remove = Remove { $label }
mineral-mnemonic-suffix = { $label } ({ $key })
mineral-carousel = Carousel
mineral-carousel-item = Item { $index } of { $count }
mineral-image-list = Images
mineral-image-list-item = Image { $index } of { $count }

mineral-sheet-bottom = Bottom sheet
mineral-sheet-side = Side sheet
mineral-sheet-resize = Resize sheet
mineral-dialog-resize = Resize dialog
mineral-sheet-handle = Drag handle
mineral-sheet-detent = Detent { $index } of { $count }

mineral-date-headline = Select date
mineral-date-range-headline = Select range
mineral-date-picker-headline = Selected date
mineral-date-picker-open = Choose date
mineral-date-input-headline = Entered date
mineral-date-invalid = Invalid format.
mineral-date-out-of-range = Out of range.
mineral-date-prev-month = Previous month
mineral-date-next-month = Next month
mineral-date-hint-year = yyyy
mineral-date-hint-month = mm
mineral-date-hint-day = dd
mineral-date-to-input = Switch to input
mineral-date-to-calendar = Switch to calendar
mineral-date-label = Date
mineral-date-start = Start date
mineral-date-end = End date

mineral-time-headline = Select time
mineral-time-input-headline = Enter time
mineral-time-hour = Hour
mineral-time-minute = Minute
mineral-time-dial = { $field } dial
mineral-time-to-dial = Switch to clock input
mineral-time-to-input = Switch to keyboard input
mineral-loading = Loading

mineral-command-placeholder = Type a command or search
mineral-command-empty = No results found.
mineral-command-results = Commands
mineral-tour-progress = Step { $index } of { $count }
mineral-tour-next = Next
mineral-tour-done = Done
mineral-refresh = Refresh

mineral-text-counter = { $count }/{ $max }
mineral-password-show = Show password
mineral-password-hide = Hide password
mineral-combobox-toggle = Show options
mineral-combobox-create = Add "{ $query }"
mineral-combobox-loading = Loading...
mineral-combobox-empty = No options
mineral-number-decrement = Decrease
mineral-number-increment = Increase
mineral-rating-value = { $value } of { $max } stars
mineral-transfer = Transfer list
mineral-transfer-source = Choices
mineral-transfer-target = Chosen
mineral-transfer-count = { $selected }/{ $total } selected
mineral-transfer-to-target = Move selected to chosen
mineral-transfer-all-to-target = Move all to chosen
mineral-transfer-to-source = Move selected back
mineral-transfer-all-to-source = Move all back
mineral-inline-submit = Save
mineral-inline-cancel = Cancel editing
mineral-file-drop = Drag files here
mineral-file-browse = Browse files
mineral-color-hex = Hex
mineral-color-hue = Hue
mineral-color-red = Red
mineral-color-green = Green
mineral-color-blue = Blue
mineral-color-swatch = Color { $hex }

mineral-pair = { $first }, { $second }
mineral-description-list = Details
mineral-timeline = Timeline
mineral-skeleton = Loading
mineral-avatar-overflow = +{ $count }
mineral-avatar-group = { $count ->
    [one] { $count } person
   *[other] { $count } people
}
mineral-avatar-more = { $count } more
mineral-alert-info = Information
mineral-alert-success = Success
mineral-alert-warning = Warning
mineral-alert-error = Error
mineral-stat-unit = { $value } { $unit }
mineral-stat-percent = { $value }%
mineral-stat-increase = Increased by { $value }
mineral-stat-decrease = Decreased by { $value }
mineral-stat-unchanged = Unchanged
mineral-pagination = Pagination
mineral-pagination-page = Page { $page }
mineral-pagination-first = First page
mineral-pagination-prev = Previous page
mineral-pagination-next = Next page
mineral-pagination-last = Last page
mineral-breadcrumbs = Breadcrumbs
mineral-breadcrumbs-more = Show path
mineral-app-bar-more = More options
mineral-breadcrumbs-current = Current page
mineral-stepper = Steps
mineral-step = Step { $number }: { $label }
mineral-step-optional = Optional
mineral-step-pending = Not started
mineral-step-active = Current step
mineral-step-complete = Completed
mineral-step-skipped = Skipped
mineral-step-error = Error
mineral-step-disabled = Unavailable
mineral-tree-loading = Loading
mineral-table-select-all = Select all rows
mineral-table-select-row = Select row
mineral-table-selection = Selection
mineral-table-sort-ascending = Sorted ascending
mineral-table-sort-descending = Sorted descending
mineral-table-resize = Resize { $column }
mineral-table-empty = No rows
mineral-table-rows-per-page = Rows per page:
mineral-table-range = { $start }-{ $end } of { $total }

# Form validation (core `form` validator codes)
mineral-required = This field is required
mineral-min-length = { $min ->
    [one] Use at least { $min } character
   *[other] Use at least { $min } characters
}
mineral-max-length = { $max ->
    [one] Use at most { $max } character
   *[other] Use at most { $max } characters
}
mineral-number = Enter a number
mineral-range-between = Enter a number between { $min } and { $max }
mineral-range-min = Enter a number of at least { $min }
mineral-range-max = Enter a number of at most { $max }
mineral-submit = Submit

# Component categories (`catalog::Category`)
mineral-category-actions = Actions
mineral-category-communication = Communication
mineral-category-containment = Containment
mineral-category-navigation = Navigation
mineral-category-selection = Selection
mineral-category-text-inputs = Text inputs
mineral-category-extras = Extras
