use nom::{named, do_parse, take, call, le_u16};

named!(
    pub parse_packet_bytes,
    do_parse!(
           ty: le_u16
        >> data: take!(ty)
        >> (
            data
        )
    )
);
