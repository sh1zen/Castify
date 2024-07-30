


pub fn parse_args(defaults: HashMap<String, Box<dyn Any>>, args: HashMap<String, Box<dyn Any>>) -> HashMap<String, String> {
    let mut options = defaults.clone();

    for arg in args {
        options.insert(arg.key, arg.value);
    }

    options
}