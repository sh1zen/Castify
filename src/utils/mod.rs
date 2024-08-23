pub mod gist;
pub mod net;

pub fn get_string_after(s: String, c: char) -> String {
    let index = s.find(c);
    if index.is_none(){
        return s;
    }
    s.clone().split_off(index.unwrap() + 1)
}