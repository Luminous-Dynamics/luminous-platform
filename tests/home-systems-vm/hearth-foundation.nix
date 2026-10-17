{
  name = "hearth-foundation-failure-isolation";

  # Permit QEMU software emulation on runners without /dev/kvm.
  requiredFeatures.kvm = false;

  nodes.hearth = { pkgs, ... }: {
    networking.hostName = "hearth";
    networking.useDHCP = false;

    # These sentinel units are test doubles, not product services. They encode
    # the architectural contract until the real Hearth NixOS module exists.
    systemd.services.hearth-core-sentinel = {
      description = "Hearth core local-service sentinel (test double)";
      wantedBy = [ "multi-user.target" ];
      after = [ "local-fs.target" ];
      serviceConfig = {
        Type = "simple";
        ExecStart = "${pkgs.coreutils}/bin/sleep infinity";
        Restart = "on-failure";
        NoNewPrivileges = true;
        PrivateTmp = true;
      };
    };

    # This unit intentionally fails on demand to simulate an optional
    # inference provider that is unavailable or broken.
    systemd.services.hearth-optional-inference-sentinel = {
      description = "Hearth optional inference sentinel (intentional failure)";
      serviceConfig = {
        Type = "oneshot";
        ExecStart = "${pkgs.coreutils}/bin/false";
        Restart = "no";
        NoNewPrivileges = true;
        PrivateTmp = true;
      };
    };
  };

  testScript = ''
    start_all()
    hearth.wait_for_unit("multi-user.target")
    hearth.wait_until_succeeds("systemctl is-active --quiet hearth-core-sentinel.service")

    # Force the optional service to fail. The failure itself is expected, but
    # it must not be a dependency of the core service.
    status, _ = hearth.execute("systemctl start hearth-optional-inference-sentinel.service")
    assert status != 0, "optional inference sentinel unexpectedly succeeded"
    hearth.succeed("systemctl is-failed --quiet hearth-optional-inference-sentinel.service")
    hearth.succeed("systemctl is-active --quiet hearth-core-sentinel.service")

    requirements = hearth.succeed(
        "systemctl show --property=Requires --value hearth-core-sentinel.service"
    )
    assert "hearth-optional-inference-sentinel.service" not in requirements, (
        "core service must not require optional inference"
    )

    # Core functionality must remain restartable after the optional failure.
    hearth.succeed("systemctl restart hearth-core-sentinel.service")
    hearth.succeed("systemctl is-active --quiet hearth-core-sentinel.service")
  '';
}
