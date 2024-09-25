pub stuct memory{}

//establish RAM as an array 327638 = 1024(number of words) * 4(number of bytes each word is) * 8 (number of bits in a byte)
RAM: [u32,327638]

//struct to read from RAM
struct read{
    base: i32
    limit: i32
}

