{ ... }: {
  perSystem = { pkgs, config, ... }:
    let crateName = "hydrus-ai-tagger";
    in {
      # declare projects
      nci.projects.${crateName}.path = ./.;
      # configure crates
      nci.crates.${crateName} = {
        profiles.release = {
          # configure features
          # features = [ "load-dynamic" ];
          # set whether to run tests or not
          runTests = true;
          # configure the main derivation for this profile's package
          drvConfig = {
            mkDerivation.buildInputs = [ pkgs.onnxruntime ];
            #   mkDerivation.preBuild = "echo starting build";
            #   env.CARGO_TERM_VERBOSE = "true";
          };
          # configure the dependencies derivation for this profile's package
          depsDrvConfig = { };
        };
      };
    };
}
