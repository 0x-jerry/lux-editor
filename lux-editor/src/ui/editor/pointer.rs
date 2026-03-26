pub fn pointer_to_line_column(
    pointer: egui::Pos2,
    line_rect: egui::Rect,
    row_height: f32,
    row_index: usize,
    char_width: f32,
) -> (usize, usize) {
    let row_delta = ((pointer.y - line_rect.top()) / row_height).floor() as isize;
    let line_index = (row_index as isize + row_delta).max(0) as usize;
    let column = ((pointer.x - line_rect.left()) / char_width)
        .floor()
        .max(0.0) as usize;
    (line_index, column)
}
