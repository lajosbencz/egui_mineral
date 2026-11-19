//! Status strip model: items packed from both edges of a bar.

/// Measured item of a strip.
#[derive(Clone, Copy, Debug)]
pub struct Item {
    /// Full width: padding, leading part and text.
    pub width: f32,
    /// Width without the text.
    pub chrome: f32,
    /// Sits at the reading end.
    pub at_end: bool,
    /// May shrink to the room left instead of being dropped.
    pub elide: bool,
}

/// Position of an item that fits.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placed {
    /// Logical offset from the item's edge.
    pub offset: f32,
    /// Allotted width.
    pub width: f32,
    /// Text room left when the item was elided.
    pub elided_room: Option<f32>,
}

/// Placement per item of a `bar` wide strip: end items claim their space first; start items that do not fit are
/// dropped with all following ones, unless the first overflowing one elides.
pub fn place(bar: f32, items: &[Item]) -> Vec<Option<Placed>> {
    let mut slots = vec![None; items.len()];
    let mut end = 0.0;
    for (i, it) in items.iter().enumerate().filter(|(_, it)| it.at_end) {
        slots[i] = Some(Placed {
            offset: end,
            width: it.width,
            elided_room: None,
        });
        end += it.width;
    }
    let mut start = 0.0;
    for (i, it) in items.iter().enumerate().filter(|(_, it)| !it.at_end) {
        let mut width = it.width;
        let mut elided_room = None;
        if start + width + end > bar {
            let room = bar - start - end - it.chrome;
            if !it.elide || room <= 0.0 {
                break;
            }
            elided_room = Some(room);
            width = bar - start - end;
        }
        slots[i] = Some(Placed {
            offset: start,
            width,
            elided_room,
        });
        start += width;
    }
    slots
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn item(width: f32, at_end: bool, elide: bool) -> Item {
        Item {
            width,
            chrome: 10.0,
            at_end,
            elide,
        }
    }

    #[test]
    fn overflowing_start_items_drop_or_elide() {
        let items = [
            item(40.0, false, false),
            item(40.0, false, false),
            item(20.0, true, false),
        ];
        let slots = place(80.0, &items);
        assert_eq!(slots[1], None, "the second start item does not fit beside the end item");
        assert_eq!(slots[2].map(|p| p.offset), Some(0.0));
        let items = [
            item(40.0, false, false),
            item(40.0, false, true),
            item(20.0, true, false),
        ];
        let slots = place(80.0, &items);
        assert_eq!(slots[1].and_then(|p| p.elided_room), Some(10.0));
    }
}
