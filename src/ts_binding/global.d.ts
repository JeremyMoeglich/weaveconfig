declare global {
    var environments: ["sample_env1", "sample_env2"];
    var mappingFromRoot: {
        [key: string]: string;
    };
    type ConfigType = {
        sample_env1: {
            sample_key1: string;
        };
        sample_env2: {
            sample_key2: string;
        };
        sample_shared: number;
    };
    type Environments = "sample_env1" | "sample_env2";
}

export type {};
