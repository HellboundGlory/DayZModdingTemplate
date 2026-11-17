class AcmeCorePerm
{
    static const string RPC_PERM = "RPC_Perm";

    void OnInit()
    {
        AcmeCoreRPC.Register("AcmeCore", RPC_PERM, this);
    }

    void RPC_Perm(CallType type, ParamsReadContext ctx, PlayerIdentity sender, Object target)
    {
        Param2<string, int> field = new Param2<string, int>("", 0);
        if (!ctx.Read(field))
        {
            return;
        }

        int role = field.param2;
        if (!AcmeCoreRPC.FitsRange("perm_role", role, 0, 7))
        {
            return;
        }
    }
}
