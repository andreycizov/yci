use nom::{named, map, do_parse, take, call, le_u16};

named!(
    pub parse_packet_bytes<Vec<u8>>,
    map!(
        do_parse!(
           ty: le_u16
            >> data: take!(ty)
            >> (
                data
            )
        ),
        Vec::from
    )
);
