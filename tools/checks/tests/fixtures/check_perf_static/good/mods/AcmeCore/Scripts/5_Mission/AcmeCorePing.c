class AcmeCorePing
{
    static const string RPC_PING = "RPC_Ping";

    void OnInit()
    {
        AcmeCoreRPC.Register("AcmeCore", RPC_PING, this);
    }

    void RPC_Ping(CallType type, ParamsReadContext ctx, PlayerIdentity sender, Object target)
    {
        Param1<string> field = new Param1<string>("");
        if (!ctx.Read(field))
        {
            return;
        }

        string line = field.param1;
        if (!AcmeCoreRPC.FitsString("line", line, 64))
        {
            return;
        }
    }
}
