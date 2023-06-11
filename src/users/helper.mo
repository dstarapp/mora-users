module {
  // local
  // public let LAUNCH_HELPER_ID = "rrkah-fqaaa-aaaaa-aaaaq-cai";
  // maintest
  // public let LAUNCH_HELPER_ID = "uzpoe-vyaaa-aaaai-qnkva-cai";

  public type CreatePlanetResp = {
    #Ok : { id : Principal };
    #Err : Text;
  };
  public type CreatePlanetSetting = {
    owner : Principal;
    code : Text;
    desc : Text;
    name : Text;
    avatar : Text;
  };
  public type LaunchHelper = actor {
    createPlanet : shared CreatePlanetSetting -> async CreatePlanetResp;
  };
  public type Self = () -> async LaunchHelper;
};
