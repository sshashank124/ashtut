struct Rng {
  uint state;
};

uint rng_uint(inout Rng rng) {
  rng.state ^= rng.state << 13;
  rng.state ^= rng.state >> 17;
  rng.state ^= rng.state << 5;
  return rng.state;
}

float rng_float(inout Rng rng) {
  return uintBitsToFloat((rng_uint(rng) >> 9) | 0x3f800000) - 1;
}
