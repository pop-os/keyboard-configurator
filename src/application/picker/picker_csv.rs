pub(super) enum PickerCsv {
    Group {
        name: String,
        cols: i32,
        width: i32,
    },
    Key {
        name: String,
        top: String,
        bottom: String,
    },
}

pub(super) fn picker_csv() -> impl Iterator<Item = PickerCsv> {
    let picker_csv = include_str!("../../../layouts/picker.csv");
    let mut is_group = true;

    let reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(picker_csv.as_bytes());

    reader.into_records().filter_map(move |record_res| {
        let record = record_res.expect("Failed to parse picker.csv");
        let name = record.get(0).unwrap_or("").to_string();

        if name.is_empty() {
            is_group = true;

            None
        } else if is_group {
            is_group = false;

            let cols_str = record.get(1).unwrap_or("");
            let cols = cols_str.parse::<i32>().unwrap_or_else(|err| {
                panic!("Failed to parse column count '{}': {}", cols_str, err)
            });

            let width_str = record.get(2).unwrap_or("");
            let width = width_str
                .parse::<i32>()
                .unwrap_or_else(|err| panic!("Failed to parse width '{}': {}", width_str, err));

            Some(PickerCsv::Group { name, cols, width })
        } else {
            let top = record.get(1).unwrap_or("").to_string();
            let bottom = record.get(2).unwrap_or("").to_string();

            Some(PickerCsv::Key { name, top, bottom })
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_picker_csv() {
        for _ in picker_csv() {}
    }
}
