var searchIndex = JSON.parse('{\
"zinoma":{"doc":"","i":[[3,"TerminationMessage","zinoma","",null,null],[5,"main","","",null,[[],["result",6]]],[5,"terminate_on_ctrlc","","",null,[[],[["result",6],["receiver",3]]]],[0,"async_utils","","",null,null],[5,"both","zinoma::async_utils","",null,[[]]],[5,"all","","Resolve as true unless any future in the iterator resolves…",null,[[]]],[0,"clean","zinoma","",null,null],[5,"clean_target_output_paths","zinoma::clean","",null,[[["target",4]]]],[5,"clean_path","","",null,[[["path",3]]]],[0,"cli","zinoma","",null,null],[5,"get_app","zinoma::cli","",null,[[],["app",3]]],[0,"arg","","",null,null],[7,"PROJECT_DIR","zinoma::cli::arg","",null,null],[7,"VERBOSITY","","",null,null],[7,"WATCH","","",null,null],[7,"CLEAN","","",null,null],[7,"GENERATE_ZSH_COMPLETION","","",null,null],[7,"TARGETS","","",null,null],[0,"config","zinoma","",null,null],[0,"ir","zinoma::config","",null,null],[3,"Config","zinoma::config::ir","",null,null],[12,"root_project_name","","",0,null],[12,"projects","","",0,null],[5,"get_dependencies","","",null,[[["target",4]],["vec",3]]],[5,"transform_target","","",null,[[["pathbuf",3],["targetid",3],["target",4]],["result",6]]],[5,"transform_input","","",null,[[["path",3],["inputresources",3],["targetid",3]],["result",6]]],[5,"transform_output","","",null,[[["path",3],["outputresources",3]],["resources",3]]],[5,"transform_extensions","","",null,[[["vec",3],["option",4]],[["btreeset",3],["option",4]]]],[11,"list_all_available_target_names","","",0,[[],[["vec",3],["string",3]]]],[11,"try_into_domain_targets","","",0,[[],[["result",6],["hashmap",3]]]],[11,"list_all_targets","","",0,[[],[["vec",3],["targetid",3]]]],[11,"get_project","","",0,[[["option",4]],["project",3]]],[0,"yaml","zinoma::config","",null,null],[3,"Config","zinoma::config::yaml","",null,null],[12,"root_project_dir","","",1,null],[12,"projects","","",1,null],[5,"canonicalize_dir","","",null,[[["path",3]],[["pathbuf",3],["result",6]]]],[5,"is_valid_target_name","","",null,[[]]],[5,"is_valid_project_name","","",null,[[]]],[0,"schema","","",null,null],[3,"Project","zinoma::config::yaml::schema","Schema of the build flow configuration file `zinoma.yml`.",null,null],[12,"targets","","Targets (aka tasks) of this project.",2,null],[12,"name","","Name of the project.",2,null],[12,"imports","","Import definitions from other Žinoma projects.",2,null],[3,"Dependencies","","List of `targets` that must complete successfully before…",null,null],[12,"0","","",3,null],[3,"InputResources","","List of artifacts that this target depends on.",null,null],[12,"0","","",4,null],[3,"OutputResources","","List of artifacts produced by this target.",null,null],[12,"0","","",5,null],[4,"Target","","A target is a command or a set of commands to run as part…",null,null],[13,"Build","","A build target represents a shell script to run as part of…",6,null],[12,"dependencies","zinoma::config::yaml::schema::Target","Dependencies of the target.",7,null],[12,"build","","The shell script to run in order to build this target.",7,null],[12,"input","","Input resources of the target.",7,null],[12,"output","","Output resources of the target.",7,null],[13,"Service","zinoma::config::yaml::schema","Service targets are useful to run scripts that do not…",6,null],[12,"dependencies","zinoma::config::yaml::schema::Target","Dependencies of the target.",8,null],[12,"service","","Shell script starting a long-lasting service.",8,null],[12,"input","","Input resources of the target.",8,null],[13,"Aggregate","zinoma::config::yaml::schema","Aggregates other targets.",6,null],[12,"dependencies","zinoma::config::yaml::schema::Target","Dependencies of the target.",9,null],[4,"InputResource","zinoma::config::yaml::schema","",null,null],[13,"DependencyOutput","","Output resources of another target.",10,null],[13,"Files","","",10,null],[12,"paths","zinoma::config::yaml::schema::InputResource","Paths to files or directories.",11,null],[12,"extensions","","Filter files resource by file extensions.",11,null],[13,"CmdStdout","zinoma::config::yaml::schema","",10,null],[12,"cmd_stdout","zinoma::config::yaml::schema::InputResource","Shell script whose output identifies the state of a…",12,null],[4,"OutputResource","zinoma::config::yaml::schema","",null,null],[13,"Files","","",13,null],[12,"paths","zinoma::config::yaml::schema::OutputResource","Paths to files or directories.",14,null],[12,"extensions","","Filter files resource by file extensions.",14,null],[13,"CmdStdout","zinoma::config::yaml::schema","",13,null],[12,"cmd_stdout","zinoma::config::yaml::schema::OutputResource","Shell script whose output identifies the state of a…",15,null],[11,"load","zinoma::config::yaml","",1,[[["path",3]],["result",6]]],[11,"load_project","","",1,[[["path",3]],[["result",6],["project",3]]]],[11,"get_project_dirs","","",1,[[],[["vec",3],["pathbuf",3]]]],[0,"domain","zinoma","",null,null],[3,"TargetMetadata","zinoma::domain","",null,null],[12,"id","","",16,null],[12,"project_dir","","",16,null],[12,"dependencies","","",16,null],[3,"BuildTarget","","",null,null],[12,"metadata","","",17,null],[12,"build_script","","",17,null],[12,"input","","",17,null],[12,"output","","",17,null],[3,"ServiceTarget","","",null,null],[12,"metadata","","",18,null],[12,"run_script","","",18,null],[12,"input","","",18,null],[3,"AggregateTarget","","",null,null],[12,"metadata","","",19,null],[3,"TargetId","","",null,null],[12,"project_name","","",20,null],[12,"target_name","","",20,null],[3,"FilesResource","","",null,null],[12,"paths","","",21,null],[12,"extensions","","",21,null],[3,"CmdResource","","",null,null],[12,"cmd","","",22,null],[12,"dir","","",22,null],[3,"Resources","","",null,null],[12,"files","","",23,null],[12,"cmds","","",23,null],[4,"Target","","",null,null],[13,"Build","","",24,null],[13,"Service","","",24,null],[13,"Aggregate","","",24,null],[5,"matches_extensions","","",null,[[["path",3],["option",4]]]],[6,"FileExtensions","","",null,null],[11,"metadata","","",24,[[],["targetmetadata",3]]],[11,"id","","",24,[[],["targetid",3]]],[11,"dependencies","","",24,[[],["vec",3]]],[11,"extend_dependencies","","",24,[[]]],[11,"input","","",24,[[],[["option",4],["resources",3]]]],[11,"output","","",24,[[],[["option",4],["resources",3]]]],[11,"extend_input","","",24,[[["resources",3]],["result",6]]],[11,"try_parse","","",20,[[["option",4]],["result",6]]],[11,"try_parse_many","","",20,[[["option",4]],[["result",6],["vec",3]]]],[11,"new","","",23,[[]]],[11,"is_empty","","",23,[[]]],[11,"extend","","",23,[[["resources",3]]]],[0,"engine","zinoma","",null,null],[4,"WatchOption","zinoma::engine","",null,null],[13,"Enabled","","",25,null],[13,"Disabled","","",25,null],[5,"run","","",null,[[["targetid",3],["targetactoroutputmessage",4],["watchoption",4],["receiver",3],["receiver",3],["terminationmessage",3],["targetactors",3],["vec",3]]]],[5,"watch","","",null,[[["terminationmessage",3],["targetactoroutputmessage",4],["targetactors",3],["receiver",3],["receiver",3]]]],[5,"execute_once","","",null,[[["terminationmessage",3],["targetactoroutputmessage",4],["targetactors",3],["receiver",3],["receiver",3]]]],[0,"builder","","",null,null],[3,"BuildCancellationMessage","zinoma::engine::builder","",null,null],[4,"BuildTerminationReport","","",null,null],[13,"Completed","","",26,null],[13,"Cancelled","","",26,null],[5,"build_target","","",null,[[["receiver",3],["buildtarget",3],["buildcancellationmessage",3]]]],[0,"incremental","zinoma::engine","",null,null],[3,"TargetEnvState","zinoma::engine::incremental","",null,null],[12,"input","","",27,null],[12,"output","","",27,null],[4,"IncrementalRunResult","","",null,null],[13,"Skipped","","",28,null],[13,"Completed","","",28,null],[13,"Cancelled","","",28,null],[5,"run","","",null,[[["resources",3],["targetmetadata",3],["option",4]]]],[5,"env_state_has_not_changed_since_last_successful_execution","","",null,[[["resources",3],["targetmetadata",3],["option",4]]]],[0,"resources_state","","",null,null],[3,"ResourcesState","zinoma::engine::incremental::resources_state","",null,null],[12,"fs","","",29,null],[12,"cmd_stdout","","",29,null],[0,"cmd_stdout","","",null,null],[3,"ResourcesState","zinoma::engine::incremental::resources_state::cmd_stdout","",null,null],[12,"0","","",30,null],[5,"get_cmd_stdout","","",null,[[["cmdresource",3]]]],[11,"current","","",30,[[]]],[11,"eq_current_state","","",30,[[]]],[0,"fs","zinoma::engine::incremental::resources_state","",null,null],[3,"ResourcesState","zinoma::engine::incremental::resources_state::fs","",null,null],[12,"0","","",31,null],[5,"get_file_modified","","",null,[[["path",3]]]],[5,"compute_file_hash","","",null,[[["path",3]]]],[11,"current","","",31,[[]]],[11,"eq_current_state","","",31,[[]]],[11,"current","zinoma::engine::incremental::resources_state","",29,[[["resources",3]]]],[11,"eq_current_state","","",29,[[["resources",3]]]],[0,"storage","zinoma::engine::incremental","",null,null],[5,"get_checksums_file_path","zinoma::engine::incremental::storage","File where the state of the target inputs and outputs are…",null,[[["targetmetadata",3]],["pathbuf",3]]],[5,"read_saved_target_env_state","","",null,[[["targetmetadata",3]]]],[5,"delete_saved_env_state","","",null,[[["targetmetadata",3]]]],[5,"save_env_state","","",null,[[["targetmetadata",3],["targetenvstate",3]]]],[11,"current","zinoma::engine::incremental","",27,[[["resources",3],["option",4]]]],[11,"eq_current_state","","",27,[[["resources",3],["option",4]]]],[0,"target_actor","zinoma::engine","",null,null],[3,"TargetActorHandleSet","zinoma::engine::target_actor","",null,null],[12,"termination_sender","","",32,null],[12,"target_actor_input_sender","","",32,null],[12,"_target_invalidated_sender","","",32,null],[12,"_watcher","","",32,null],[4,"ActorInputMessage","","",null,null],[13,"Requested","","Indicates the execution of the build scripts or services…",33,null],[12,"kind","zinoma::engine::target_actor::ActorInputMessage","",34,null],[12,"requester","","",34,null],[13,"Unrequested","zinoma::engine::target_actor","Indicates the execution of the build scripts or services…",33,null],[12,"kind","zinoma::engine::target_actor::ActorInputMessage","",35,null],[12,"requester","","",35,null],[13,"Ok","zinoma::engine::target_actor","Indicates the execution of the build scripts behind the…",33,null],[12,"kind","zinoma::engine::target_actor::ActorInputMessage","",36,null],[12,"target_id","","",36,null],[12,"actual","","",36,null],[13,"Invalidated","zinoma::engine::target_actor","Indicates the build scripts or services behind the target…",33,null],[12,"kind","zinoma::engine::target_actor::ActorInputMessage","",37,null],[12,"target_id","","",37,null],[4,"TargetActorOutputMessage","zinoma::engine::target_actor","",null,null],[13,"TargetExecutionError","","",38,null],[13,"MessageActor","","",38,null],[12,"dest","zinoma::engine::target_actor::TargetActorOutputMessage","",39,null],[12,"msg","","",39,null],[4,"ActorId","zinoma::engine::target_actor","",null,null],[13,"Root","","",40,null],[13,"Target","","",40,null],[4,"ExecutionKind","","",null,null],[13,"Build","","",41,null],[13,"Service","","",41,null],[5,"launch_target_actor","","",null,[[["target",4],["sender",3],["targetactoroutputmessage",4],["watchoption",4]],["result",6]]],[0,"aggregate_target_actor","","",null,null],[3,"AggregateTargetActor","zinoma::engine::target_actor::aggregate_target_actor","",null,null],[12,"_target","","",42,null],[12,"helper","","",42,null],[11,"new","","",42,[[["targetactorhelper",3],["aggregatetarget",3]]]],[11,"run","","",42,[[]]],[0,"build_target_actor","zinoma::engine::target_actor","",null,null],[3,"BuildTargetActor","zinoma::engine::target_actor::build_target_actor","",null,null],[12,"target","","",43,null],[12,"helper","","",43,null],[11,"new","","",43,[[["buildtarget",3],["targetactorhelper",3]]]],[11,"run","","",43,[[]]],[0,"service_target_actor","zinoma::engine::target_actor","",null,null],[3,"ServiceTargetActor","zinoma::engine::target_actor::service_target_actor","",null,null],[12,"target","","",44,null],[12,"helper","","",44,null],[12,"service_process","","",44,null],[11,"new","","",44,[[["servicetarget",3],["targetactorhelper",3]]]],[11,"run","","",44,[[]]],[11,"stop_service","","",44,[[]]],[11,"restart_service","","",44,[[]]],[0,"target_actor_helper","zinoma::engine::target_actor","",null,null],[3,"TargetActorHelper","zinoma::engine::target_actor::target_actor_helper","",null,null],[12,"target_id","","",45,null],[12,"termination_events","","",45,null],[12,"target_invalidated_events","","",45,null],[12,"target_actor_input_receiver","","",45,null],[12,"target_actor_output_sender","","",45,null],[12,"to_execute","","",45,null],[12,"executed","","",45,null],[12,"dependencies","","",45,null],[12,"unavailable_dependencies","","",45,null],[12,"requesters","","",45,null],[11,"new","","",45,[[["actorinputmessage",4],["targetactoroutputmessage",4],["receiver",3],["targetinvalidatedmessage",3],["receiver",3],["terminationmessage",3],["sender",3],["receiver",3],["targetmetadata",3]]]],[11,"should_execute","","",45,[[["executionkind",4]]]],[11,"notify_invalidated","","",45,[[["executionkind",4]]]],[11,"set_execution_started","","",45,[[]]],[11,"notify_execution_failed","","",45,[[["error",3]]]],[11,"send_to_actor","","",45,[[["actorid",4],["actorinputmessage",4]]]],[11,"send_to_dependencies","","",45,[[["actorinputmessage",4]]]],[11,"send_to_requesters","","",45,[[["executionkind",4],["actorinputmessage",4]]]],[11,"notify_success","","",45,[[["executionkind",4]]]],[11,"request_dependencies","","",45,[[["executionkind",4]]]],[11,"handle_unrequested","","",45,[[["executionkind",4],["actorid",4]]]],[11,"unrequest_dependencies","","",45,[[["executionkind",4]]]],[0,"target_actors","zinoma::engine","",null,null],[3,"TargetActors","zinoma::engine::target_actors","",null,null],[12,"targets","","",46,null],[12,"target_actor_output_sender","","",46,null],[12,"watch_option","","",46,null],[12,"target_actor_handles","","",46,null],[12,"target_actor_join_handles","","",46,null],[11,"new","","",46,[[["targetid",3],["target",4],["sender",3],["targetactoroutputmessage",4],["watchoption",4],["hashmap",3]]]],[11,"get_target_actor_handles","","",46,[[["targetid",3]],[["result",6],["targetactorhandleset",3]]]],[11,"send","","",46,[[["actorinputmessage",4],["targetid",3]]]],[11,"request_target","","",46,[[["targetid",3]]]],[11,"terminate","","",46,[[]]],[11,"send_termination_message","","",46,[[["hashmap",3]]]],[0,"watcher","zinoma::engine","",null,null],[3,"TargetWatcher","zinoma::engine::watcher","",null,null],[12,"_watchers","","",47,null],[3,"TargetInvalidatedMessage","","",null,null],[5,"is_tmp_editor_file","","",null,[[["path",3]]]],[11,"new","","",47,[[["sender",3],["option",4],["targetid",3],["resources",3]],[["result",6],["option",4]]]],[11,"build_immediate_watcher","","",47,[[["targetid",3],["sender",3],["btreeset",3],["option",4],["targetinvalidatedmessage",3]],[["result",6],["recommendedwatcher",6]]]],[0,"fs","zinoma","",null,null],[5,"list_files_in_resources","zinoma::fs","",null,[[]]],[5,"list_files_in_paths","","",null,[[["option",4]]]],[5,"list_files_in_path","","",null,[[["path",3],["option",4]]]],[0,"run_script","zinoma","",null,null],[5,"build_command","zinoma::run_script","",null,[[["path",3]],["command",3]]],[0,"work_dir","zinoma","",null,null],[5,"is_in_work_dir","zinoma::work_dir","",null,[[["path",3]]]],[5,"get_work_dir_path","","",null,[[["path",3]],["pathbuf",3]]],[5,"remove_work_dir","","",null,[[["path",3]]]],[5,"is_work_dir","","",null,[[["direntry",3]]]],[17,"WORK_DIR_NAME","","Name of the directory in which Žinoma stores its own files.",null,null],[7,"GLOBAL","zinoma","",null,null],[7,"DEFAULT_CHANNEL_CAP","","",null,null],[11,"from","","",48,[[]]],[11,"into","","",48,[[]]],[11,"borrow","","",48,[[]]],[11,"borrow_mut","","",48,[[]]],[11,"try_from","","",48,[[],["result",4]]],[11,"try_into","","",48,[[],["result",4]]],[11,"type_id","","",48,[[],["typeid",3]]],[11,"from","zinoma::config::ir","",0,[[]]],[11,"into","","",0,[[]]],[11,"borrow","","",0,[[]]],[11,"borrow_mut","","",0,[[]]],[11,"try_from","","",0,[[],["result",4]]],[11,"try_into","","",0,[[],["result",4]]],[11,"type_id","","",0,[[],["typeid",3]]],[11,"from","zinoma::config::yaml","",1,[[]]],[11,"into","","",1,[[]]],[11,"borrow","","",1,[[]]],[11,"borrow_mut","","",1,[[]]],[11,"try_from","","",1,[[],["result",4]]],[11,"try_into","","",1,[[],["result",4]]],[11,"type_id","","",1,[[],["typeid",3]]],[11,"from","zinoma::config::yaml::schema","",2,[[]]],[11,"into","","",2,[[]]],[11,"borrow","","",2,[[]]],[11,"borrow_mut","","",2,[[]]],[11,"try_from","","",2,[[],["result",4]]],[11,"try_into","","",2,[[],["result",4]]],[11,"type_id","","",2,[[],["typeid",3]]],[11,"from","","",3,[[]]],[11,"into","","",3,[[]]],[11,"borrow","","",3,[[]]],[11,"borrow_mut","","",3,[[]]],[11,"try_from","","",3,[[],["result",4]]],[11,"try_into","","",3,[[],["result",4]]],[11,"type_id","","",3,[[],["typeid",3]]],[11,"from","","",4,[[]]],[11,"into","","",4,[[]]],[11,"borrow","","",4,[[]]],[11,"borrow_mut","","",4,[[]]],[11,"try_from","","",4,[[],["result",4]]],[11,"try_into","","",4,[[],["result",4]]],[11,"type_id","","",4,[[],["typeid",3]]],[11,"from","","",5,[[]]],[11,"into","","",5,[[]]],[11,"borrow","","",5,[[]]],[11,"borrow_mut","","",5,[[]]],[11,"try_from","","",5,[[],["result",4]]],[11,"try_into","","",5,[[],["result",4]]],[11,"type_id","","",5,[[],["typeid",3]]],[11,"from","","",6,[[]]],[11,"into","","",6,[[]]],[11,"borrow","","",6,[[]]],[11,"borrow_mut","","",6,[[]]],[11,"try_from","","",6,[[],["result",4]]],[11,"try_into","","",6,[[],["result",4]]],[11,"type_id","","",6,[[],["typeid",3]]],[11,"from","","",10,[[]]],[11,"into","","",10,[[]]],[11,"borrow","","",10,[[]]],[11,"borrow_mut","","",10,[[]]],[11,"try_from","","",10,[[],["result",4]]],[11,"try_into","","",10,[[],["result",4]]],[11,"type_id","","",10,[[],["typeid",3]]],[11,"from","","",13,[[]]],[11,"into","","",13,[[]]],[11,"borrow","","",13,[[]]],[11,"borrow_mut","","",13,[[]]],[11,"try_from","","",13,[[],["result",4]]],[11,"try_into","","",13,[[],["result",4]]],[11,"type_id","","",13,[[],["typeid",3]]],[11,"from","zinoma::domain","",16,[[]]],[11,"into","","",16,[[]]],[11,"to_owned","","",16,[[]]],[11,"clone_into","","",16,[[]]],[11,"to_string","","",16,[[],["string",3]]],[11,"borrow","","",16,[[]]],[11,"borrow_mut","","",16,[[]]],[11,"try_from","","",16,[[],["result",4]]],[11,"try_into","","",16,[[],["result",4]]],[11,"type_id","","",16,[[],["typeid",3]]],[11,"__clone_box","","",16,[[["private",3]]]],[11,"from","","",17,[[]]],[11,"into","","",17,[[]]],[11,"to_string","","",17,[[],["string",3]]],[11,"borrow","","",17,[[]]],[11,"borrow_mut","","",17,[[]]],[11,"try_from","","",17,[[],["result",4]]],[11,"try_into","","",17,[[],["result",4]]],[11,"type_id","","",17,[[],["typeid",3]]],[11,"from","","",18,[[]]],[11,"into","","",18,[[]]],[11,"to_string","","",18,[[],["string",3]]],[11,"borrow","","",18,[[]]],[11,"borrow_mut","","",18,[[]]],[11,"try_from","","",18,[[],["result",4]]],[11,"try_into","","",18,[[],["result",4]]],[11,"type_id","","",18,[[],["typeid",3]]],[11,"from","","",19,[[]]],[11,"into","","",19,[[]]],[11,"borrow","","",19,[[]]],[11,"borrow_mut","","",19,[[]]],[11,"try_from","","",19,[[],["result",4]]],[11,"try_into","","",19,[[],["result",4]]],[11,"type_id","","",19,[[],["typeid",3]]],[11,"from","","",20,[[]]],[11,"into","","",20,[[]]],[11,"to_owned","","",20,[[]]],[11,"clone_into","","",20,[[]]],[11,"to_string","","",20,[[],["string",3]]],[11,"borrow","","",20,[[]]],[11,"borrow_mut","","",20,[[]]],[11,"try_from","","",20,[[],["result",4]]],[11,"try_into","","",20,[[],["result",4]]],[11,"type_id","","",20,[[],["typeid",3]]],[11,"equivalent","","",20,[[]]],[11,"__clone_box","","",20,[[["private",3]]]],[11,"from","","",21,[[]]],[11,"into","","",21,[[]]],[11,"to_owned","","",21,[[]]],[11,"clone_into","","",21,[[]]],[11,"borrow","","",21,[[]]],[11,"borrow_mut","","",21,[[]]],[11,"try_from","","",21,[[],["result",4]]],[11,"try_into","","",21,[[],["result",4]]],[11,"type_id","","",21,[[],["typeid",3]]],[11,"__clone_box","","",21,[[["private",3]]]],[11,"from","","",22,[[]]],[11,"into","","",22,[[]]],[11,"to_owned","","",22,[[]]],[11,"clone_into","","",22,[[]]],[11,"borrow","","",22,[[]]],[11,"borrow_mut","","",22,[[]]],[11,"try_from","","",22,[[],["result",4]]],[11,"try_into","","",22,[[],["result",4]]],[11,"type_id","","",22,[[],["typeid",3]]],[11,"__clone_box","","",22,[[["private",3]]]],[11,"from","","",23,[[]]],[11,"into","","",23,[[]]],[11,"to_owned","","",23,[[]]],[11,"clone_into","","",23,[[]]],[11,"borrow","","",23,[[]]],[11,"borrow_mut","","",23,[[]]],[11,"try_from","","",23,[[],["result",4]]],[11,"try_into","","",23,[[],["result",4]]],[11,"type_id","","",23,[[],["typeid",3]]],[11,"__clone_box","","",23,[[["private",3]]]],[11,"from","","",24,[[]]],[11,"into","","",24,[[]]],[11,"to_string","","",24,[[],["string",3]]],[11,"borrow","","",24,[[]]],[11,"borrow_mut","","",24,[[]]],[11,"try_from","","",24,[[],["result",4]]],[11,"try_into","","",24,[[],["result",4]]],[11,"type_id","","",24,[[],["typeid",3]]],[11,"from","zinoma::engine","",25,[[]]],[11,"into","","",25,[[]]],[11,"to_owned","","",25,[[]]],[11,"clone_into","","",25,[[]]],[11,"borrow","","",25,[[]]],[11,"borrow_mut","","",25,[[]]],[11,"try_from","","",25,[[],["result",4]]],[11,"try_into","","",25,[[],["result",4]]],[11,"type_id","","",25,[[],["typeid",3]]],[11,"__clone_box","","",25,[[["private",3]]]],[11,"from","zinoma::engine::builder","",49,[[]]],[11,"into","","",49,[[]]],[11,"borrow","","",49,[[]]],[11,"borrow_mut","","",49,[[]]],[11,"try_from","","",49,[[],["result",4]]],[11,"try_into","","",49,[[],["result",4]]],[11,"type_id","","",49,[[],["typeid",3]]],[11,"from","","",26,[[]]],[11,"into","","",26,[[]]],[11,"borrow","","",26,[[]]],[11,"borrow_mut","","",26,[[]]],[11,"try_from","","",26,[[],["result",4]]],[11,"try_into","","",26,[[],["result",4]]],[11,"type_id","","",26,[[],["typeid",3]]],[11,"from","zinoma::engine::incremental","",27,[[]]],[11,"into","","",27,[[]]],[11,"borrow","","",27,[[]]],[11,"borrow_mut","","",27,[[]]],[11,"try_from","","",27,[[],["result",4]]],[11,"try_into","","",27,[[],["result",4]]],[11,"type_id","","",27,[[],["typeid",3]]],[11,"from","","",28,[[]]],[11,"into","","",28,[[]]],[11,"borrow","","",28,[[]]],[11,"borrow_mut","","",28,[[]]],[11,"try_from","","",28,[[],["result",4]]],[11,"try_into","","",28,[[],["result",4]]],[11,"type_id","","",28,[[],["typeid",3]]],[11,"from","zinoma::engine::incremental::resources_state","",29,[[]]],[11,"into","","",29,[[]]],[11,"borrow","","",29,[[]]],[11,"borrow_mut","","",29,[[]]],[11,"try_from","","",29,[[],["result",4]]],[11,"try_into","","",29,[[],["result",4]]],[11,"type_id","","",29,[[],["typeid",3]]],[11,"from","zinoma::engine::incremental::resources_state::cmd_stdout","",30,[[]]],[11,"into","","",30,[[]]],[11,"borrow","","",30,[[]]],[11,"borrow_mut","","",30,[[]]],[11,"try_from","","",30,[[],["result",4]]],[11,"try_into","","",30,[[],["result",4]]],[11,"type_id","","",30,[[],["typeid",3]]],[11,"from","zinoma::engine::incremental::resources_state::fs","",31,[[]]],[11,"into","","",31,[[]]],[11,"borrow","","",31,[[]]],[11,"borrow_mut","","",31,[[]]],[11,"try_from","","",31,[[],["result",4]]],[11,"try_into","","",31,[[],["result",4]]],[11,"type_id","","",31,[[],["typeid",3]]],[11,"from","zinoma::engine::target_actor","",32,[[]]],[11,"into","","",32,[[]]],[11,"borrow","","",32,[[]]],[11,"borrow_mut","","",32,[[]]],[11,"try_from","","",32,[[],["result",4]]],[11,"try_into","","",32,[[],["result",4]]],[11,"type_id","","",32,[[],["typeid",3]]],[11,"from","","",33,[[]]],[11,"into","","",33,[[]]],[11,"to_owned","","",33,[[]]],[11,"clone_into","","",33,[[]]],[11,"borrow","","",33,[[]]],[11,"borrow_mut","","",33,[[]]],[11,"try_from","","",33,[[],["result",4]]],[11,"try_into","","",33,[[],["result",4]]],[11,"type_id","","",33,[[],["typeid",3]]],[11,"__clone_box","","",33,[[["private",3]]]],[11,"from","","",38,[[]]],[11,"into","","",38,[[]]],[11,"borrow","","",38,[[]]],[11,"borrow_mut","","",38,[[]]],[11,"try_from","","",38,[[],["result",4]]],[11,"try_into","","",38,[[],["result",4]]],[11,"type_id","","",38,[[],["typeid",3]]],[11,"from","","",40,[[]]],[11,"into","","",40,[[]]],[11,"to_owned","","",40,[[]]],[11,"clone_into","","",40,[[]]],[11,"borrow","","",40,[[]]],[11,"borrow_mut","","",40,[[]]],[11,"try_from","","",40,[[],["result",4]]],[11,"try_into","","",40,[[],["result",4]]],[11,"type_id","","",40,[[],["typeid",3]]],[11,"equivalent","","",40,[[]]],[11,"__clone_box","","",40,[[["private",3]]]],[11,"from","","",41,[[]]],[11,"into","","",41,[[]]],[11,"to_owned","","",41,[[]]],[11,"clone_into","","",41,[[]]],[11,"borrow","","",41,[[]]],[11,"borrow_mut","","",41,[[]]],[11,"try_from","","",41,[[],["result",4]]],[11,"try_into","","",41,[[],["result",4]]],[11,"type_id","","",41,[[],["typeid",3]]],[11,"equivalent","","",41,[[]]],[11,"__clone_box","","",41,[[["private",3]]]],[11,"from","zinoma::engine::target_actor::aggregate_target_actor","",42,[[]]],[11,"into","","",42,[[]]],[11,"borrow","","",42,[[]]],[11,"borrow_mut","","",42,[[]]],[11,"try_from","","",42,[[],["result",4]]],[11,"try_into","","",42,[[],["result",4]]],[11,"type_id","","",42,[[],["typeid",3]]],[11,"from","zinoma::engine::target_actor::build_target_actor","",43,[[]]],[11,"into","","",43,[[]]],[11,"borrow","","",43,[[]]],[11,"borrow_mut","","",43,[[]]],[11,"try_from","","",43,[[],["result",4]]],[11,"try_into","","",43,[[],["result",4]]],[11,"type_id","","",43,[[],["typeid",3]]],[11,"from","zinoma::engine::target_actor::service_target_actor","",44,[[]]],[11,"into","","",44,[[]]],[11,"borrow","","",44,[[]]],[11,"borrow_mut","","",44,[[]]],[11,"try_from","","",44,[[],["result",4]]],[11,"try_into","","",44,[[],["result",4]]],[11,"type_id","","",44,[[],["typeid",3]]],[11,"from","zinoma::engine::target_actor::target_actor_helper","",45,[[]]],[11,"into","","",45,[[]]],[11,"borrow","","",45,[[]]],[11,"borrow_mut","","",45,[[]]],[11,"try_from","","",45,[[],["result",4]]],[11,"try_into","","",45,[[],["result",4]]],[11,"type_id","","",45,[[],["typeid",3]]],[11,"from","zinoma::engine::target_actors","",46,[[]]],[11,"into","","",46,[[]]],[11,"borrow","","",46,[[]]],[11,"borrow_mut","","",46,[[]]],[11,"try_from","","",46,[[],["result",4]]],[11,"try_into","","",46,[[],["result",4]]],[11,"type_id","","",46,[[],["typeid",3]]],[11,"from","zinoma::engine::watcher","",47,[[]]],[11,"into","","",47,[[]]],[11,"borrow","","",47,[[]]],[11,"borrow_mut","","",47,[[]]],[11,"try_from","","",47,[[],["result",4]]],[11,"try_into","","",47,[[],["result",4]]],[11,"type_id","","",47,[[],["typeid",3]]],[11,"from","","",50,[[]]],[11,"into","","",50,[[]]],[11,"borrow","","",50,[[]]],[11,"borrow_mut","","",50,[[]]],[11,"try_from","","",50,[[],["result",4]]],[11,"try_into","","",50,[[],["result",4]]],[11,"type_id","","",50,[[],["typeid",3]]],[11,"from","zinoma::config::ir","",0,[[["config",3]]]],[11,"from","zinoma::engine","",25,[[]]],[11,"clone","zinoma::domain","",16,[[],["targetmetadata",3]]],[11,"clone","","",20,[[],["targetid",3]]],[11,"clone","","",21,[[],["filesresource",3]]],[11,"clone","","",22,[[],["cmdresource",3]]],[11,"clone","","",23,[[],["resources",3]]],[11,"clone","zinoma::engine::target_actor","",33,[[],["actorinputmessage",4]]],[11,"clone","","",40,[[],["actorid",4]]],[11,"clone","","",41,[[],["executionkind",4]]],[11,"clone","zinoma::engine","",25,[[],["watchoption",4]]],[11,"default","zinoma::config::yaml::schema","",3,[[],["dependencies",3]]],[11,"default","","",4,[[],["inputresources",3]]],[11,"default","","",5,[[],["outputresources",3]]],[11,"eq","zinoma::domain","",20,[[["targetid",3]]]],[11,"ne","","",20,[[["targetid",3]]]],[11,"eq","","",21,[[["filesresource",3]]]],[11,"ne","","",21,[[["filesresource",3]]]],[11,"eq","","",22,[[["cmdresource",3]]]],[11,"ne","","",22,[[["cmdresource",3]]]],[11,"eq","","",23,[[["resources",3]]]],[11,"ne","","",23,[[["resources",3]]]],[11,"eq","zinoma::engine::incremental::resources_state::cmd_stdout","",30,[[["resourcesstate",3]]]],[11,"ne","","",30,[[["resourcesstate",3]]]],[11,"eq","zinoma::engine::incremental::resources_state::fs","",31,[[["resourcesstate",3]]]],[11,"ne","","",31,[[["resourcesstate",3]]]],[11,"eq","zinoma::engine::incremental::resources_state","",29,[[["resourcesstate",3]]]],[11,"ne","","",29,[[["resourcesstate",3]]]],[11,"eq","zinoma::engine::incremental","",28,[[["incrementalrunresult",4]]]],[11,"eq","","",27,[[["targetenvstate",3]]]],[11,"ne","","",27,[[["targetenvstate",3]]]],[11,"eq","zinoma::engine::target_actor","",40,[[["actorid",4]]]],[11,"ne","","",40,[[["actorid",4]]]],[11,"eq","","",41,[[["executionkind",4]]]],[11,"fmt","zinoma::config::yaml::schema","",2,[[["formatter",3]],["result",6]]],[11,"fmt","","",6,[[["formatter",3]],["result",6]]],[11,"fmt","","",10,[[["formatter",3]],["result",6]]],[11,"fmt","","",13,[[["formatter",3]],["result",6]]],[11,"fmt","","",3,[[["formatter",3]],["result",6]]],[11,"fmt","","",4,[[["formatter",3]],["result",6]]],[11,"fmt","","",5,[[["formatter",3]],["result",6]]],[11,"fmt","zinoma::config::yaml","",1,[[["formatter",3]],["result",6]]],[11,"fmt","zinoma::domain","",16,[[["formatter",3]],["result",6]]],[11,"fmt","","",17,[[["formatter",3]],["result",6]]],[11,"fmt","","",18,[[["formatter",3]],["result",6]]],[11,"fmt","","",19,[[["formatter",3]],["result",6]]],[11,"fmt","","",24,[[["formatter",3]],["result",6]]],[11,"fmt","","",20,[[["formatter",3]],["result",6]]],[11,"fmt","","",21,[[["formatter",3]],["result",6]]],[11,"fmt","","",22,[[["formatter",3]],["result",6]]],[11,"fmt","","",23,[[["formatter",3]],["result",6]]],[11,"fmt","zinoma::engine::target_actor","",33,[[["formatter",3]],["result",6]]],[11,"fmt","","",38,[[["formatter",3]],["result",6]]],[11,"fmt","","",40,[[["formatter",3]],["result",6]]],[11,"fmt","","",41,[[["formatter",3]],["result",6]]],[11,"fmt","zinoma::domain","",16,[[["formatter",3]],["result",6]]],[11,"fmt","","",17,[[["formatter",3]],["result",6]]],[11,"fmt","","",18,[[["formatter",3]],["result",6]]],[11,"fmt","","",24,[[["formatter",3]],["result",6]]],[11,"fmt","","",20,[[["formatter",3]],["result",6]]],[11,"hash","","",20,[[]]],[11,"hash","zinoma::engine::target_actor","",40,[[]]],[11,"hash","","",41,[[]]],[11,"schema_name","zinoma::config::yaml::schema","",2,[[],["string",3]]],[11,"json_schema","","",2,[[["schemagenerator",3]],["schema",4]]],[11,"schema_name","","",6,[[],["string",3]]],[11,"json_schema","","",6,[[["schemagenerator",3]],["schema",4]]],[11,"schema_name","","",10,[[],["string",3]]],[11,"json_schema","","",10,[[["schemagenerator",3]],["schema",4]]],[11,"schema_name","","",13,[[],["string",3]]],[11,"json_schema","","",13,[[["schemagenerator",3]],["schema",4]]],[11,"schema_name","","",3,[[],["string",3]]],[11,"json_schema","","",3,[[["schemagenerator",3]],["schema",4]]],[11,"schema_name","","",4,[[],["string",3]]],[11,"json_schema","","",4,[[["schemagenerator",3]],["schema",4]]],[11,"schema_name","","",5,[[],["string",3]]],[11,"json_schema","","",5,[[["schemagenerator",3]],["schema",4]]],[11,"deserialize","","",2,[[],["result",4]]],[11,"deserialize","","",6,[[],["result",4]]],[11,"deserialize","","",10,[[],["result",4]]],[11,"deserialize","","",13,[[],["result",4]]],[11,"deserialize","","",3,[[],["result",4]]],[11,"deserialize","","",4,[[],["result",4]]],[11,"deserialize","","",5,[[],["result",4]]],[11,"deserialize","zinoma::engine::incremental::resources_state::cmd_stdout","",30,[[],["result",4]]],[11,"deserialize","zinoma::engine::incremental::resources_state::fs","",31,[[],["result",4]]],[11,"deserialize","zinoma::engine::incremental::resources_state","",29,[[],["result",4]]],[11,"deserialize","zinoma::engine::incremental","",27,[[],["result",4]]],[11,"serialize","zinoma::config::yaml::schema","",2,[[],["result",4]]],[11,"serialize","","",6,[[],["result",4]]],[11,"serialize","","",10,[[],["result",4]]],[11,"serialize","","",13,[[],["result",4]]],[11,"serialize","","",3,[[],["result",4]]],[11,"serialize","","",4,[[],["result",4]]],[11,"serialize","","",5,[[],["result",4]]],[11,"serialize","zinoma::engine::incremental::resources_state::cmd_stdout","",30,[[],["result",4]]],[11,"serialize","zinoma::engine::incremental::resources_state::fs","",31,[[],["result",4]]],[11,"serialize","zinoma::engine::incremental::resources_state","",29,[[],["result",4]]],[11,"serialize","zinoma::engine::incremental","",27,[[],["result",4]]]],"p":[[3,"Config"],[3,"Config"],[3,"Project"],[3,"Dependencies"],[3,"InputResources"],[3,"OutputResources"],[4,"Target"],[13,"Build"],[13,"Service"],[13,"Aggregate"],[4,"InputResource"],[13,"Files"],[13,"CmdStdout"],[4,"OutputResource"],[13,"Files"],[13,"CmdStdout"],[3,"TargetMetadata"],[3,"BuildTarget"],[3,"ServiceTarget"],[3,"AggregateTarget"],[3,"TargetId"],[3,"FilesResource"],[3,"CmdResource"],[3,"Resources"],[4,"Target"],[4,"WatchOption"],[4,"BuildTerminationReport"],[3,"TargetEnvState"],[4,"IncrementalRunResult"],[3,"ResourcesState"],[3,"ResourcesState"],[3,"ResourcesState"],[3,"TargetActorHandleSet"],[4,"ActorInputMessage"],[13,"Requested"],[13,"Unrequested"],[13,"Ok"],[13,"Invalidated"],[4,"TargetActorOutputMessage"],[13,"MessageActor"],[4,"ActorId"],[4,"ExecutionKind"],[3,"AggregateTargetActor"],[3,"BuildTargetActor"],[3,"ServiceTargetActor"],[3,"TargetActorHelper"],[3,"TargetActors"],[3,"TargetWatcher"],[3,"TerminationMessage"],[3,"BuildCancellationMessage"],[3,"TargetInvalidatedMessage"]]}\
}');
addSearchOptions(searchIndex);initSearch(searchIndex);