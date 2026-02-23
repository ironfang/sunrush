pub struct Bus {

}

impl Bus {
    pub fn new() -> Self {
        Bus {}
    }

    pub fn send(&self, data: &[u8]) {
        // Send data to the bus
    }

    pub fn subscribe(&self, callback: fn(&[u8])) {
        // Subscribe to the bus and call the callback when data is received
    }
}

trait Reciever<T> {
    fn on_message(&self, msg: T);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
