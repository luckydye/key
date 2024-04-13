static PASSWORD_CHARSET: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz\
    0123456789!@#$%^&*()_+-=[]{}|;':,.<>?";

pub fn generate_password(length: &usize) -> String {
  random_string::generate(*length, PASSWORD_CHARSET)
}
