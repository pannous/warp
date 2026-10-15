mod test_algo_dijkstra;
mod test_algo_levenshtein;
mod test_algo_life;
mod test_algo_queens;
mod test_algo_sieve;
mod test_algo_sorting;
mod test_all_samples;
#[cfg(feature = "native")] // the paint window's input
mod test_brush_ripples; // card shader-click-ripples
mod test_calculator_fixes;
mod test_cli_help;
mod test_code_golf; // card code-golf
mod test_data_structure_fixes;
mod test_draw;
mod test_draw_label; // card paint-text
mod test_file_spectrum; // card gpu-visualizer-mp3
mod test_gpu_draw; // card gpu-draw
mod test_hints_toggle; // card hints-toggle
mod test_json_parser_fixes;
mod test_kitchensink;
mod test_live_redefine; // card live-redefine
mod test_midi;
mod test_music_values; // card music-values
mod test_note_values; // card note-value
mod test_paint;
mod test_paint_function; // card gpu-functions
mod test_paint_in_window; // card paint-keeps
mod test_paint_lists;
mod test_paint_shader; // P234
mod test_paint_window_ends; // card exit-key
mod test_plain_hints_when_piped;
mod test_playground_samples;
mod test_program_cache; // card cache-compiled-shader
mod test_samples;
mod test_samples_run_cleanly;
mod test_save_painting; // card save-painting
mod test_shader_blocks; // card g_oFJc
mod test_shader_builtins; // card shader-builtins
mod test_shader_errors; // card wgsl-error
mod test_shader_holes; // card shader-holes
mod test_shader_system_values;
mod test_small_samples; // cards text-samples, error-examples
mod test_sound;
mod test_sound_aliasing; // card reason-beep
mod test_sound_at; // card sample-accurate
mod test_sound_at_in_blocks; // card inside-loop
mod test_sound_buses; // card shared-buses
mod test_sound_clock; // card sound-pro
mod test_sound_effects; // card signal-graph
mod test_sound_engine; // card native-sound
mod test_sound_fades; // card sound-gain-ramps
mod test_sound_files; // card sound-library
mod test_sound_filters; // card synth-filters
mod test_sound_handles; // card sound-pro
mod test_sound_notes; // card sound-pro
mod test_sound_pan; // card stereo-pan
mod test_sound_patterns; // card patterns-live
mod test_sound_pipes; // card sound-pipe-graph
mod test_sound_render; // card sound-pro
mod test_sound_samples; // card sound-samples
mod test_sound_signal; // card real-time
mod test_sound_spectrum; // card winamp-like
mod test_sound_synth; // card sound-pro
mod test_sound_tempo; // card sound-pro
mod test_sound_tempo_float; // card exact-tempo-growth
mod test_sound_use; // card bizarre-global
pub(crate) mod test_sound_voices; // card sound-pro; its rendered helper serves scope too
mod test_sound_wavs; // card sound-wavs
mod test_task_sound; // card task-sound
mod test_warp_command; // card midi-out // card values-number
mod test_window_size; // card make-width
