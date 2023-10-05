#[allow(dead_code)]
pub fn get_closest<T>(t: f32, t_max: f32, vec: &Vec<(f32, T)>) -> Option<(usize, &(f32, T), bool)> {
    let mut closest_obj: Option<&(f32, T)> = None;
    let mut closest_index = 0;
    let mut smallest_diff = t_max;
    for (i, obj) in vec.iter().enumerate() {
        let diff = (t - obj.0).abs();
        if diff < smallest_diff {
            closest_index = i;
            closest_obj = Some(obj);
            smallest_diff = diff;
        }
    }

    if let Some(closest_obj) = closest_obj {
        let is_overestimate = &closest_obj.0 > &t;
        Some((closest_index, &closest_obj, is_overestimate))
    } else {
        None
    }
}
