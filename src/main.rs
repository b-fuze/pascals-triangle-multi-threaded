use std::env::args;
use std::thread;
use std::sync::Arc;
use std::fmt::Write;
use crossbeam::channel::bounded;

#[derive(Clone)]
struct Workload {
    previous_row: Arc<Vec<u64>>,
    offset: usize,
    row_index: usize,
}

#[derive(Clone)]
struct ThreadOutput {
    thread_index: usize,
    output_size: usize,
    output_start: usize,
    output_vec: Arc<Vec<u64>>,
    output_string: Arc<String>,
}

const THREAD_BATCH_SIZE: usize = 100;
const OUTPUT_BUFFER_MAX_SIZE: usize = 1024 * 1024 * 500; // 500MiB

fn main() {
    let rows = args()
        .nth(1).expect("missing Pascal triangle row count")
        .parse::<usize>().expect("row count must be a positive integer");
    let num_cpus = num_cpus::get();
    // let num_cpus = 1; // Uncomment to force "single threaded"
    let (output_channel, output_channel_recv) = bounded::<ThreadOutput>(num_cpus);
    let mut input_channels = (0..num_cpus)
        .into_iter()
        .map(|_| {
            let (channel, recv) = bounded::<Option<Workload>>(1);
            (channel, Some(recv))
        })
        .collect::<Vec<_>>();

    let threads = (0..num_cpus)
        .into_iter()
        .map(|index| {
            let output_channel = output_channel.clone();
            let input_channel = input_channels[index].1.take().unwrap();
            thread::spawn(move || {
                let mut output = Arc::new(vec![0; THREAD_BATCH_SIZE + 1]);
                let mut output_string = Arc::new(String::new());
                while let Some(workload) = input_channel.recv().unwrap() {
                    let mut output_index = 0;
                    let start;
                    let orig_offset;

                    {
                        let Workload {
                            previous_row,
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

                        let output_string_mut = Arc::get_mut(&mut output_string).unwrap();
                        output_string_mut.clear();
                        write!(output_string_mut, "{:?}", &output_vec[0..output_index]).unwrap();
                    }

                    output_channel
                        .send(ThreadOutput {
                            thread_index: index,
                            output_start: start - orig_offset,
                            output_size: output_index,
                            output_vec: Arc::clone(&output),
                            output_string: Arc::clone(&output_string),
                        })
                        .unwrap();
                }
            })
        })
        .collect::<Vec<_>>();

    let mut console_output = String::new();
    let mut previous_row = vec![0; rows + 1];
    // First two numbers of previous_row are 1, the rest 0
    previous_row[0..2].copy_from_slice(&[1, 1]);
    let mut previous_row = Arc::new(previous_row);
    let mut thread_output_reports: Vec<Option<ThreadOutput>> = vec![None; num_cpus];
    let mut write_log = Vec::with_capacity(num_cpus * THREAD_BATCH_SIZE);
    let mut write_log_row_offset = 0;

    write!(&mut console_output, "{:?}\n", [1]).unwrap();
    write!(&mut console_output, "{:?}\n", [1, 1]).unwrap();

    for i in 3..=rows {
        write!(&mut console_output, "[1").unwrap();
        let mut row_offset = 0;

        while row_offset < i {
            let mut thread_index = 0;
            let current_initial_row_offset = row_offset;
            {
                let mut workload = Workload {
                    previous_row: Arc::clone(&previous_row),
                    offset: 0,
                    row_index: i,
                };

                for _ in 0..num_cpus {
                    workload.offset = row_offset;
                    input_channels[thread_index].0.send(Some(workload.clone())).unwrap();

                    row_offset += THREAD_BATCH_SIZE;
                    thread_index += 1;
                    if row_offset >= i {
                        break;
                    }
                }
            }

            // Wait for all the threads to finish
            for _ in 0..thread_index {
                let thread_output = output_channel_recv.recv().unwrap();
                let thread_index = thread_output.thread_index;
                thread_output_reports[thread_index] = Some(thread_output);
            }

            // Write previous log
            if write_log.len() > 0 {
                let previous_row_mut = Arc::get_mut(&mut previous_row).unwrap();
                (&mut previous_row_mut[write_log_row_offset..write_log_row_offset + write_log.len()])
                    .copy_from_slice(&write_log);
                write_log.clear();
            }

            {
                for index in 0..thread_index {
                    let cur_thread_output = thread_output_reports[index].take().unwrap();
                    let output_vec = cur_thread_output.output_vec;

                    let write_log_len = write_log.len();
                    write_log.resize(write_log_len + cur_thread_output.output_size, 0);
                    (&mut write_log[write_log_len..write_log_len + cur_thread_output.output_size])
                        .copy_from_slice(&output_vec[0..cur_thread_output.output_size]);

                    write_log_row_offset = (current_initial_row_offset + cur_thread_output.output_start).max(
                        write_log_row_offset
                    );

                    let output_string = &cur_thread_output.output_string;
                    if output_string.len() > 2 {
                        write!(&mut console_output, ", ").unwrap();
                    }
                    write!(&mut console_output, "{}", &output_string[1..output_string.len() - 1]).unwrap();
                }
            }

            // Flush string if it gets too big
            if console_output.len() > OUTPUT_BUFFER_MAX_SIZE {
                print!("{}", &console_output);
                console_output.clear();
            }
        }

        // Write previous log
        if write_log.len() > 0 {
            let previous_row_mut = Arc::get_mut(&mut previous_row).unwrap();
            (&mut previous_row_mut[write_log_row_offset..write_log_row_offset + write_log.len()])
                .copy_from_slice(&write_log);
            write_log.clear();
        }

        write_log_row_offset = 0;
        write!(&mut console_output, "]\n").unwrap();
    }

    // Print output
    print!("{}", &console_output);

    // Shut down threads
    for index in 0..num_cpus {
        input_channels[index].0.send(None).unwrap();
    }

    for thread in threads {
        thread.join().unwrap();
    }
}
