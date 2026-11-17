class AcmeCoreTicker
{
    void OnTick(float timeslice)
    {
        DoWork();
    }

    void DoWork()
    {
        int count = CountThings();
    }

    int CountThings()
    {
        int total = 0;
        for (int i = 0; i < 10; i++)
        {
            total = total + i;
        }
        return total;
    }
}
