pub fn dumb_intersection<T: Clone + PartialEq>(aa: &[T], bb: &[T]) -> Vec<T> {
    let mut res = Vec::with_capacity(aa.len().max(bb.len()));
    for a in aa.iter() {
        for b in bb.iter() {
            if a == b { res.push(a.clone()) }
        }
    }
    res
}

