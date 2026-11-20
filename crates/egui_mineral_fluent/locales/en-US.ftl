# egui_mineral_fluent component strings (Fluent UI React v9 wording).
# Translations: same ids in `<locale>.ftl`, registered with `egui_mineral::i18n::add_resource`
# or provided by the app localizer.

mineral-split-button-menu = More options
mineral-required-mark = *

mineral-percent = { NUMBER($value, maximumFractionDigits: 0) }%

# Core messages (`egui_mineral::core_messages`)
mineral-mnemonic-suffix = { $label } ({ $key })
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

# Component categories (`catalog::Category`)
mineral-category-actions = Actions
mineral-category-inputs = Inputs
mineral-category-feedback = Feedback
mineral-category-surfaces = Surfaces
mineral-category-navigation = Navigation
mineral-category-data = Data
mineral-category-extras = Extras

# Inputs
mineral-range-value = { $start } - { $end }
mineral-range-start = { $label } start
mineral-range-end = { $label } end
mineral-search-clear = Clear text
mineral-tag-picker-remove = Remove { $label }
mineral-calendar-previous-month = Go to previous month
mineral-calendar-next-month = Go to next month
mineral-calendar-previous-year = Go to previous year
mineral-calendar-next-year = Go to next year
mineral-calendar-previous-years = Previous year range
mineral-calendar-next-years = Next year range
mineral-calendar-go-to-today = Go to today
mineral-rating-value = { $value } of { $max }
mineral-info-label-info = information
mineral-progress = Progress
mineral-skeleton-loading = Loading Content
mineral-presence-available = Available
mineral-presence-away = Away
mineral-presence-busy = Busy
mineral-presence-do-not-disturb = Do not disturb
mineral-presence-offline = Offline
mineral-presence-out-of-office = Out of office
mineral-presence-unknown = Unknown
mineral-presence-blocked = Blocked
mineral-presence-with-out-of-office = { $status }, out of office
mineral-dismiss = Dismiss
mineral-dialog-close = Close
mineral-drawer-close = Close
mineral-window-close = Close window
mineral-teaching-previous = Previous
mineral-teaching-next = Next
mineral-teaching-done = Done
mineral-teaching-step = Step { $step }
mineral-card = Card
mineral-card-select = Select card
mineral-accordion = Accordion
mineral-list = List
mineral-avatar-active = active
mineral-avatar-inactive = inactive
mineral-avatar-group-more = View more people.
mineral-avatar-group-overflow = Overflow
mineral-tag-dismiss = Dismiss { $label }
mineral-carousel-previous = Previous slide
mineral-carousel-next = Next slide
mineral-carousel-autoplay = Autoplay
mineral-table-select-all = Select all rows
mineral-table-select-row = Select row
mineral-table-sorted-ascending = { $label }, sorted ascending
mineral-table-sorted-descending = { $label }, sorted descending

# Extras
mineral-pagination = Pagination
mineral-pagination-first = First page
mineral-pagination-prev = Previous page
mineral-pagination-next = Next page
mineral-pagination-last = Last page
mineral-menu-strip = Menu bar
mineral-command-placeholder = Search commands
mineral-command-empty = No results found.
mineral-command-results = Commands
mineral-submit = Submit
