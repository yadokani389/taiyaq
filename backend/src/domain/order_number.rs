use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub struct DisplayOrderNumber(u8);

impl DisplayOrderNumber {
    pub fn from_order_id(order_id: u32) -> Self {
        Self((order_id % 100) as u8)
    }

    pub fn as_str(self) -> String {
        format!("{:02}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::DisplayOrderNumber;

    #[test]
    fn formats_order_id_as_two_digit_reusable_number() {
        let cases = [(1, "01"), (23, "23"), (100, "00"), (123, "23")];

        for (order_id, expected) in cases {
            assert_eq!(
                DisplayOrderNumber::from_order_id(order_id).as_str(),
                expected
            );
        }
    }
}
