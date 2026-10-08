//! Ordered highlight normalization, independent of parsing and UI state.
use gpui::HighlightStyle;
use std::{collections::{BTreeMap,BTreeSet},ops::Range};

pub(crate) fn unique_styles(
    total_range: &Range<usize>,
    styles: Vec<(Range<usize>, HighlightStyle)>,
) -> Vec<(Range<usize>, HighlightStyle)> {
    if styles.is_empty() {
        return styles;
    }

    let mut intervals = BTreeSet::new();
    let mut significant_intervals = BTreeSet::new();

    // For example
    //
    // from: [(6..11), (6..11), (11..17), (17..25), (16..19), (25..59))]
    // to:   [6, 11, 16, 17, 19, 25, 59]
    intervals.insert(total_range.start);
    intervals.insert(total_range.end);
    for (range, _) in &styles {
        intervals.insert(range.start);
        intervals.insert(range.end);
        significant_intervals.insert(range.end); // End points are significant for merging decisions
    }

    let intervals: Vec<usize> = intervals.into_iter().collect();
    let mut result = Vec::with_capacity(intervals.len().saturating_sub(1));

    // Sweep interval endpoints instead of scanning every highlight for every span.
    // Active captures stay in their original order, preserving per-field precedence.
    let mut events: BTreeMap<usize, Vec<(usize,bool)>> = BTreeMap::new();
    for (index,(range,_)) in styles.iter().enumerate() {
        if range.start<range.end {
            events.entry(range.start).or_default().push((index,true));
            events.entry(range.end).or_default().push((index,false));
        }
    }
    let mut active=BTreeSet::new();
    for pair in intervals.windows(2) {
        let interval=pair[0]..pair[1];
        if let Some(changes)=events.get(&interval.start) {
            for &(index,starting) in changes {if !starting{active.remove(&index);}}
            for &(index,starting) in changes {if starting{active.insert(index);}}
        }
        if interval.is_empty(){continue;}
        let mut top_style=None;
        for &index in &active {
            let style=styles[index].1;
            if let Some(current)=&mut top_style {merge_highlight_style(current,&style);}else{top_style=Some(style);}
        }
        result.push((interval,top_style.unwrap_or_default()));
    }

    // Merge adjacent ranges with the same style, but not across significant boundaries
    let mut merged: Vec<(Range<usize>, HighlightStyle)> = Vec::with_capacity(result.len());
    for (range, style) in result {
        if let Some((last_range, last_style)) = merged.last_mut() {
            if last_range.end == range.start
                && *last_style == style
                && !significant_intervals.contains(&range.start)
            {
                // Merge adjacent ranges with same style, but not across significant boundaries
                last_range.end = range.end;
                continue;
            }
        }
        merged.push((range, style));
    }

    merged
}

/// Merge other style (Other on top)
fn merge_highlight_style(style: &mut HighlightStyle, other: &HighlightStyle) {
    if let Some(color) = other.color {
        style.color = Some(color);
    }
    if let Some(font_weight) = other.font_weight {
        style.font_weight = Some(font_weight);
    }
    if let Some(font_style) = other.font_style {
        style.font_style = Some(font_style);
    }
    if let Some(background_color) = other.background_color {
        style.background_color = Some(background_color);
    }
    if let Some(underline) = other.underline {
        style.underline = Some(underline);
    }
    if let Some(strikethrough) = other.strikethrough {
        style.strikethrough = Some(strikethrough);
    }
    if let Some(fade_out) = other.fade_out {
        style.fade_out = Some(fade_out);
    }
}

#[cfg(test)]
mod tests {
    use gpui::Hsla;

    use super::*;
    use crate::Colorize as _;

    fn color_style(color: Hsla) -> HighlightStyle {
        let mut style = HighlightStyle::default();
        style.color = Some(color);
        style
    }

    #[track_caller]
    fn assert_unique_styles(
        range: Range<usize>,
        left: Vec<(Range<usize>, HighlightStyle)>,
        right: Vec<(Range<usize>, HighlightStyle)>,
    ) {
        fn color_name(c: Option<Hsla>) -> String {
            match c {
                Some(c) => {
                    if c == gpui::red() {
                        "red".to_string()
                    } else if c == gpui::green() {
                        "green".to_string()
                    } else if c == gpui::blue() {
                        "blue".to_string()
                    } else {
                        c.to_hex()
                    }
                }
                None => "clean".to_string(),
            }
        }

        let left = unique_styles(&range, left);
        if left.len() != right.len() {
            println!("\n---------------------------------------------");
            for (range, style) in left.iter() {
                println!("({:?}, {})", range, color_name(style.color));
            }
            println!("---------------------------------------------");
            panic!("left {} styles, right {} styles", left.len(), right.len());
        }
        for (left, right) in left.into_iter().zip(right) {
            if left.1.color != right.1.color || left.0 != right.0 {
                panic!(
                    "\n left: ({:?}, {})\nright: ({:?}, {})\n",
                    left.0,
                    color_name(left.1.color),
                    right.0,
                    color_name(right.1.color)
                );
            }
        }
    }

    #[test]
    fn sweep_matches_overlapping_capture_precedence() {
        fn reference(total:&Range<usize>,styles:Vec<(Range<usize>,HighlightStyle)>)->Vec<(Range<usize>,HighlightStyle)>{
            let mut points=BTreeSet::from([total.start,total.end]);let mut ends=BTreeSet::new();
            for (r,_) in &styles{points.insert(r.start);points.insert(r.end);ends.insert(r.end);}
            let points:Vec<_>=points.into_iter().collect();let mut out:Vec<(Range<usize>,HighlightStyle)>=Vec::new();
            for p in points.windows(2){let range=p[0]..p[1];let mut style=None;for (r,s) in &styles{if r.start<=range.start&&range.end<=r.end{if let Some(current)=&mut style{merge_highlight_style(current,s);}else{style=Some(*s);}}}
                let style=style.unwrap_or_default();if let Some((prev,old))=out.last_mut(){if prev.end==range.start&&*old==style&&!ends.contains(&range.start){prev.end=range.end;continue;}}out.push((range,style));
            }out
        }
        let mut seed=1u64;for _ in 0..256{let mut next=||{seed=seed.wrapping_mul(6364136223846793005).wrapping_add(1);(seed>>32)as usize};let styles=(0..20).map(|i|{let start=next()%32;let end=start+next()%(33-start);let mut style=HighlightStyle::default();if i%3==0{style.color=Some(gpui::red());}if i%3==1{style.color=Some(gpui::green());style.font_weight=Some(gpui::FontWeight::BOLD);}if i%3==2{style.font_style=Some(gpui::FontStyle::Italic);} (start..end,style)}).collect::<Vec<_>>();assert_eq!(unique_styles(&(0..32),styles.clone()),reference(&(0..32),styles));}
    }

    #[test]
    fn test_unique_styles() {
        let red = color_style(gpui::red());
        let green = color_style(gpui::green());
        let blue = color_style(gpui::blue());
        let clean = HighlightStyle::default();

        assert_unique_styles(
            0..65,
            vec![
                (2..10, clean),
                (2..10, clean),
                (5..11, red),
                (2..6, clean),
                (10..15, green),
                (15..30, clean),
                (29..35, blue),
                (35..40, green),
                (45..60, blue),
            ],
            vec![
                (0..5, clean),
                (5..6, red),
                (6..10, red),
                (10..11, green),
                (11..15, green),
                (15..29, clean),
                (29..30, blue),
                (30..35, blue),
                (35..40, green),
                (40..45, clean),
                (45..60, blue),
                (60..65, clean),
            ],
        );
    }
}
