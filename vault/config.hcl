ui = true

storage "file" {
  path = "/vault/file"
}

listener "tcp" {
  address = "0.0.0.0:41224",
  tls_disable = true
}
