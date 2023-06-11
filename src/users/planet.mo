module {
  public type PlanetActor = actor {
    verifySubcriber : shared(p : Principal) -> async Bool;
    verifyOwnerWriter : shared(p : Principal) -> async Bool;
  }
}