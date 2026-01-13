use pallas::crypto::key::ed25519;

pub trait Ed25519Signer {
    fn public_key(&self) -> ed25519::PublicKey;
    fn sign<T: AsRef<[u8]>>(&self, msg: T) -> ed25519::Signature;
}

impl Ed25519Signer for ed25519::SecretKey {
    fn public_key(&self) -> ed25519::PublicKey {
        self.public_key()
    }

    fn sign<T: AsRef<[u8]>>(&self, msg: T) -> ed25519::Signature {
        self.sign(msg)
    }
}

impl Ed25519Signer for ed25519::SecretKeyExtended {
    fn public_key(&self) -> ed25519::PublicKey {
        self.public_key()
    }

    fn sign<T: AsRef<[u8]>>(&self, msg: T) -> ed25519::Signature {
        self.sign(msg)
    }
}
