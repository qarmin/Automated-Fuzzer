def match_type( param_name, all_param_values, configs, validate_uris=True, search_paths=True, search_subdirs=True, allow_remote_uri=True, allow_local_path=True, input_dir=""):
    type_matching_functions = { 
    }
    assert not ((value[0] == "null" or value[0] == "") and 
        not configs[param_name]["null_allowed"]
    ), "parameter is \"null\" but this is not allowed."
    assert not (
        not configs[param_name]["is_array"]# fmt: on and 
        (value[0] == "itemNull")
    ), "Parameter is set to \"itemNull\" but it is not an array/list."