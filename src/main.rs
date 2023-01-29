use std::env::args;
use std::thread;
use std::sync::{Arc, RwLock};
#[cfg(not(feature = "no-output"))]
use std::fmt::Write;
use crossbeam::channel::{bounded, unbounded};

#[derive(Clone)]
struct Workload {
    offset: usize,
    row_index: usize,
}

#[derive(Clone)]
struct ThreadReport {
    thread_index: usize,
    output_size: usize,
    output_start: usize,
    output_vec: Arc<Vec<u64>>,
}

#[cfg(not(feature = "no-output"))]
enum StdoutOutput {
    Start {
        thread_count: usize,
        has_begin: bool,
        has_end: bool,
    },
    End,
    Output {
        thread_index: usize,
        output_string: String,
    },
}

const THREAD_BATCH_SIZE: usize = 500;

fn main() {
    let rows = args()
        .nth(1).expect("missing Pascal triangle row count")
        .parse::<usize>().expect("row count must be a positive integer");
    let num_cpus = num_cpus::get();
    // let num_cpus = 1; // Uncomment to force "single threaded"

    #[cfg(not(feature = "no-output"))]
    let (stdout_channel, stdout_channel_recv) = unbounded::<StdoutOutput>();
    #[cfg(not(feature = "no-output"))]
    let stdout_thread = thread::spawn(move || {
        let mut thread_outputs = vec![None; num_cpus];
        let mut thread_count = 0;
        let mut thread_count_orig = 0;
        let mut has_end = false;

        loop {
            match stdout_channel_recv.recv().unwrap() {
                StdoutOutput::Start {
                    thread_count: count,
                    has_end: end,
                    has_begin,
                } => {
                    thread_count = count;
                    thread_count_orig = count;
                    has_end = end;

                    if has_begin {
                        print!("[1");
                    }
                },
                StdoutOutput::End => { break },
                StdoutOutput::Output {
                    thread_index,
                    output_string,
                } => {
                    thread_count -= 1;
                    thread_outputs[thread_index] = Some(output_string);

                    if thread_count == 0 {
                        for thread_index in 0..thread_count_orig {
                            let output_string = thread_outputs[thread_index].take().unwrap();

                            if output_string.len() > 2 {
                                print!(", ");
                            }
                            print!("{}", &output_string[1..output_string.len() - 1]);
                        }

                        if has_end {
                            println!("]");
                        }
                    }
                },
            };
        }
    });

    let mut previous_row = vec![0; rows + 1];
    // First two numbers of previous_row are 1, the rest 0
    previous_row[0..2].copy_from_slice(&[1, 1]);
    let previous_row = Arc::new(RwLock::new(previous_row));

    let mut previous_row_refs = (0..num_cpus)
        .into_iter()
        .map(|_| Some(Arc::clone(&previous_row)))
        .collect::<Vec<_>>();
    #[cfg(not(feature = "no-output"))]
    let mut output_channels = (0..num_cpus)
        .into_iter()
        .map(|_| Some(stdout_channel.clone()))
        .collect::<Vec<_>>();
    let mut input_channels = (0..num_cpus)
        .into_iter()
        .map(|_| {
            let (channel, recv) = bounded::<Option<Workload>>(1);
            (channel, Some(recv))
        })
        .collect::<Vec<_>>();
    let (output_channel, output_channel_recv) = unbounded::<ThreadReport>();

    let threads = (0..num_cpus)
        .into_iter()
        .map(|index| {
            let previous_row = previous_row_refs[index].take().unwrap();
            let sync_output_channel = output_channel.clone();
            #[cfg(not(feature = "no-output"))]
            let output_channel = output_channels[index].take().unwrap();
            let input_channel = input_channels[index].1.take().unwrap();
            thread::spawn(move || {
                let mut output = Arc::new(vec![0; THREAD_BATCH_SIZE + 1]);
                #[cfg(not(feature = "no-output"))]
                let mut output_string = String::new();
                while let Some(workload) = input_channel.recv().unwrap() {
                    let mut output_index = 0;
                    let start;
                    let orig_offset;

                    {
                        let previous_row = previous_row.read().unwrap();
                        let Workload {
                            offset,
                            row_index,
                        } = workload;
                        orig_offset = offset;
                        start = offset.max(1);

                        let end = (offset + THREAD_BATCH_SIZE).min(row_index);

                        let output_vec = Arc::get_mut(&mut output).unwrap();
                        for i in start..end {
                            output_vec[output_index] = previous_row[i - 1] + previous_row[i];
                            output_index += 1;
                        }

                        #[cfg(not(feature = "no-output"))]
                        {
                            output_string.clear();
                            write!(output_string, "{:?}", &output_vec[0..output_index]).unwrap();
                        }
                    }

                    #[cfg(not(feature = "no-output"))]
                    output_channel
                        .send(StdoutOutput::Output {
                            thread_index: index,
                            output_string: output_string.clone(),
                        })
                        .unwrap();
                    sync_output_channel
                        .send(ThreadReport {
                            thread_index: index,
                            output_start: start - orig_offset,
                            output_size: output_index,
                            output_vec: Arc::clone(&output),
                        })
                        .unwrap();
                }
            })
        })
        .collect::<Vec<_>>();

    // let mut console_output_size = 0;
    let mut thread_output_reports: Vec<Option<ThreadReport>> = vec![None; num_cpus];
    let mut write_log = vec![0; num_cpus * THREAD_BATCH_SIZE];
    let mut write_log_row_offset = 0;
    let mut write_log_len = 0;

    #[cfg(not(feature = "no-output"))]
    {
        println!("{:?}", [1]);
        println!("{:?}", [1, 1]);
    }

    for i in 3..=rows {
        let mut row_offset = 0;

        while row_offset < i {

            let thread_count;
            let current_initial_row_offset = row_offset;
            {
                thread_count = ((i as i64 - row_offset as i64) / (THREAD_BATCH_SIZE as i64))
                    .max(1)
                    .min(num_cpus as i64) as usize;
                #[cfg(not(feature = "no-output"))]
                let has_end = row_offset + THREAD_BATCH_SIZE * num_cpus >= i;
                #[cfg(not(feature = "no-output"))]
                stdout_channel.send(StdoutOutput::Start {
                    thread_count,
                    has_begin: row_offset == 0,
                    has_end,
                }).unwrap();

                let mut workload = Workload {
                    offset: 0,
                    row_index: i,
                };

                for thread_index in 0..thread_count {
                    workload.offset = row_offset;
                    input_channels[thread_index].0.send(Some(workload.clone())).unwrap();

                    row_offset += THREAD_BATCH_SIZE;
                }
            }

            // Wait for all the threads to finish
            for _ in 0..thread_count {
                let thread_output = output_channel_recv.recv().unwrap();
                let thread_index = thread_output.thread_index;
                thread_output_reports[thread_index] = Some(thread_output);
            }

            // Write previous log
            if write_log_len > 0 {
                let mut previous_row_mut = previous_row.write().unwrap();
                (&mut previous_row_mut[write_log_row_offset..write_log_row_offset + write_log_len])
                    .copy_from_slice(&write_log[0..write_log_len]);
                write_log_len = 0;
            }

            for index in 0..thread_count {
                let cur_thread_output = thread_output_reports[index].take().unwrap();
                let output_vec = cur_thread_output.output_vec;

                let new_write_log_len = write_log_len + cur_thread_output.output_size;
                (&mut write_log[write_log_len..new_write_log_len])
                    .copy_from_slice(&output_vec[0..cur_thread_output.output_size]);

                write_log_len = new_write_log_len;
                write_log_row_offset = (current_initial_row_offset + cur_thread_output.output_start).max(
                    write_log_row_offset
                );
            }
        }

        // Write previous log
        if write_log_len > 0 {
            let mut previous_row_mut = previous_row.write().unwrap();
            (&mut previous_row_mut[write_log_row_offset..write_log_row_offset + write_log_len])
                .copy_from_slice(&write_log[0..write_log_len]);
            write_log_len = 0;
        }

        write_log_row_offset = 0;
    }

    // Shut down threads
    for index in 0..num_cpus {
        input_channels[index].0.send(None).unwrap();
    }
    #[cfg(not(feature = "no-output"))]
    stdout_channel.send(StdoutOutput::End).unwrap();

    for thread in threads {
        thread.join().unwrap();
    }
    #[cfg(not(feature = "no-output"))]
    stdout_thread.join().unwrap();
}
