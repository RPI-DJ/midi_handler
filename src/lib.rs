use std::convert::From;
use std::sync::mpsc::sync_channel;
use std::io;
use jack::Client;

const MAX_MIDI: usize = 3;

#[derive(Copy, Clone)]
pub struct MidiCopy {
    len: usize,
    data: [u8; MAX_MIDI],
    time: jack::Frames,
}

impl From<jack::RawMidi<'_>> for MidiCopy {
    fn from(midi: jack::RawMidi<'_>) -> Self {
        let len = std::cmp::min(MAX_MIDI, midi.bytes.len());
        let mut data = [0; MAX_MIDI];
        data[..len].copy_from_slice(&midi.bytes[..len]);
        MidiCopy{
            len,
            data,
            time: midi.time,
        }
    }
}

impl std::fmt::Debug for MidiCopy {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "Midi {{ time: {}, len {}, data: {:?} }}",
            self.time,
            self.len,
            &self.data[..self.len]
            )
    }
}

pub fn midi_choice(client: &Client) -> String {
    println!("Select a port");
    let ports = client.ports(None, None, jack::PortFlags::IS_OUTPUT);
    let mut i = 0;
    for port in &ports {
        println!("{}: {}", i, port);
        i += 1;
    }

    let mut raw_port_choice = String::new();
    io::stdin().read_line(&mut raw_port_choice).ok();
    let index = raw_port_choice.trim().parse::<usize>().unwrap();

    return ports[index].clone(); 
} 

pub fn create_client(client_name: String) -> Client {
    let (client, _status) = Client::new(&client_name, jack::ClientOptions::NO_START_SERVER).unwrap();
    return client
}

pub fn midi_init(sender: std::sync::mpsc::SyncSender<MidiCopy>, signal_reciver: std::sync::mpsc::Receiver<u8>) {
    println!("initializing midi");
    let client = create_client("rdj".to_string());

    let choice = midi_choice(&client);

    let midi_in = client.register_port("rdj_midi_in", jack::MidiIn::default())
        .unwrap();


    let cback = move |c: &jack::Client, ps: &jack::ProcessScope| -> jack::Control {
        let source_port = c.port_by_name(&choice).unwrap();
        let shower_name = midi_in.name().unwrap();
        if !source_port.is_connected_to(&shower_name).unwrap() {
            c.connect_ports(&source_port, &midi_in).unwrap();
        }
        let show_p = midi_in.iter(ps);
        for e in show_p {
            let c: MidiCopy = e.into();
            match sender.try_send(c) {
                Ok(_) => println!("sent successful"),
                Err(e) => panic!("Error sending message: {:?}", e),
            };

        }

        jack::Control::Continue
    };

    std::thread::spawn(move || {
        let active_client = client.activate_async((), jack::ClosureProcessHandler::new(cback)).unwrap();
        if let Ok(_) = signal_reciver.recv() {
            active_client.deactivate().unwrap();
        }
    });

}
